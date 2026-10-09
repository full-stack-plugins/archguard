//! Linux descriptor-relative snapshot copy: no source path is reopened recursively.
use super::{Budget, Failure};
use std::{
    ffi::{CStr, CString},
    fs::{self, File},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    },
    path::{Component, Path},
    time::Instant,
};
fn input(error: impl std::fmt::Display) -> Failure {
    Failure::Input(error.to_string())
}
fn open_at(parent: &File, name: &CStr, directory: bool) -> Result<File, Failure> {
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: parent owns a valid fd and name is NUL-terminated. Successful
    // descriptor is transferred exactly once to File for RAII ownership.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(input(std::io::Error::last_os_error()));
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn open_directory(source: &Path) -> Result<File, Failure> {
    let mut dir = File::open("/").map_err(input)?;
    for part in source.components() {
        match part {
            Component::RootDir => (),
            Component::Normal(name) => {
                let name = CString::new(name.as_bytes()).map_err(input)?;
                dir = open_at(&dir, &name, true)?;
            }
            _ => return Err(input("source must be an absolute canonical directory")),
        }
    }
    Ok(dir)
}
pub(super) fn tree(
    source: &Path,
    destination: &Path,
    budget: &Budget,
    started: Instant,
) -> Result<(), Failure> {
    State {
        budget,
        started,
        bytes: 0,
        entries: 0,
    }
    .directory(&open_directory(source)?, destination, 0)
}
struct State<'a> {
    budget: &'a Budget,
    started: Instant,
    bytes: u64,
    entries: usize,
}
impl State<'_> {
    fn deadline(&self) -> Result<(), Failure> {
        if self.started.elapsed() >= self.budget.timeout {
            Err(Failure::Timeout)
        } else {
            Ok(())
        }
    }
    fn directory(&mut self, dir: &File, destination: &Path, depth: usize) -> Result<(), Failure> {
        self.deadline()?;
        if depth > self.budget.depth {
            return Err(input("input depth budget exceeded"));
        }
        // procfs path is generated only from this owned fd, never candidate text.
        // It pins enumeration even if the source directory is renamed or replaced.
        for entry in fs::read_dir(format!("/proc/self/fd/{}", dir.as_raw_fd())).map_err(input)? {
            self.deadline()?;
            let name = entry.map_err(input)?.file_name();
            if name == ".git" || name == "target" {
                continue;
            }
            self.entries += 1;
            if self.entries > self.budget.files {
                return Err(input("input entry budget exceeded"));
            }
            let c_name = CString::new(name.as_bytes()).map_err(input)?;
            let mut file = open_at(dir, &c_name, false)?;
            let metadata = file.metadata().map_err(input)?;
            let dest = destination.join(&name);
            if metadata.is_dir() {
                fs::create_dir(&dest).map_err(input)?;
                self.directory(&file, &dest, depth + 1)?;
            } else if metadata.is_file() {
                if metadata.len() > self.budget.file_bytes
                    || metadata.len() > self.budget.total_bytes - self.bytes
                {
                    return Err(input("input byte budget exceeded"));
                }
                let mut output = File::create(dest).map_err(input)?;
                let mut copied = 0u64;
                loop {
                    self.deadline()?;
                    let mut buffer = [0; 8192];
                    // Include at most one sentinel byte beyond the remaining
                    // allowance. The budget is validated before subtraction.
                    let left =
                        (self.budget.file_bytes - copied).min(self.budget.total_bytes - self.bytes);
                    let want = ((left + 1).min(buffer.len() as u64)) as usize;
                    let count = file.read(&mut buffer[..want]).map_err(input)?;
                    if count == 0 {
                        break;
                    }
                    if count as u64 > left {
                        return Err(input("input byte budget exceeded"));
                    }
                    copied += count as u64;
                    self.bytes += count as u64;
                    output.write_all(&buffer[..count]).map_err(input)?;
                }
            } else {
                return Err(input("symlink or special input file is unsupported"));
            }
        }
        self.deadline()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_directory_is_not_redirected_by_a_symlink_replacement() {
        let source = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        let child = source.path().join("child");
        fs::create_dir(&child).unwrap();
        fs::write(child.join("inside"), b"owned").unwrap();
        fs::write(external.path().join("outside"), b"must not copy").unwrap();
        let pinned = open_directory(&child).unwrap();
        fs::rename(&child, source.path().join("moved")).unwrap();
        std::os::unix::fs::symlink(external.path(), &child).unwrap();
        let budget = Budget::default();
        State {
            budget: &budget,
            started: Instant::now(),
            bytes: 0,
            entries: 0,
        }
        .directory(&pinned, dest.path(), 0)
        .unwrap();
        assert_eq!(fs::read(dest.path().join("inside")).unwrap(), b"owned");
        assert!(!dest.path().join("outside").exists());
        assert!(open_directory(&child).is_err());
    }
}
