//! Bounded subprocess and isolated-input helpers. These are not an OS sandbox.
//! The declaration entrypoint accepts only fixed Cargo/rustc tools and rejects
//! configurations that could select extra executables or external input paths.
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct Budget {
    pub timeout: Duration,
    pub output_bytes: usize,
    pub file_bytes: u64,
    pub total_bytes: u64,
    pub files: usize,
    pub depth: usize,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(20),
            output_bytes: 8 * 1024 * 1024,
            file_bytes: 16 * 1024 * 1024,
            total_bytes: 128 * 1024 * 1024,
            files: 10_000,
            depth: 64,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    Input(String),
    Timeout,
    OutputLimit,
    Exit(Option<i32>),
    Start(String),
    UnsupportedPlatform,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Failure {}

/// Caller supplies an already trusted executable, arguments and environment.
/// This primitive grants no authority to arbitrary candidate-selected tools.
#[cfg(unix)]
pub fn run(
    tool: &Path,
    args: &[OsString],
    cwd: &Path,
    environment: &BTreeMap<OsString, OsString>,
    budget: &Budget,
) -> Result<Vec<u8>, Failure> {
    use std::os::unix::process::CommandExt;
    let mut child = Command::new(tool)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| Failure::Start(e.to_string()))?;
    use std::os::fd::AsRawFd;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let mut output = Vec::new();
    let mut size = 0usize;
    let start = Instant::now();
    let result = (|| {
        for fd in [stdout.as_raw_fd(), stderr.as_raw_fd()] {
            // SAFETY: both descriptors belong to the live pipe objects above.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags == -1
                || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1
            {
                return Err(Failure::Input(std::io::Error::last_os_error().to_string()));
            }
        }
        let mut stdout_eof = false;
        let mut stderr_eof = false;
        loop {
            if start.elapsed() >= budget.timeout {
                return Err(Failure::Timeout);
            }
            let mut progressed = false;
            for (pipe, eof, retain) in [
                (&mut stdout as &mut dyn Read, &mut stdout_eof, true),
                (&mut stderr as &mut dyn Read, &mut stderr_eof, false),
            ] {
                if *eof {
                    continue;
                }
                let mut buffer = [0; 8192];
                match pipe.read(&mut buffer) {
                    Ok(0) => *eof = true,
                    Ok(count) => {
                        progressed = true;
                        size = size.saturating_add(count);
                        if size > budget.output_bytes {
                            return Err(Failure::OutputLimit);
                        }
                        if retain {
                            output.extend_from_slice(&buffer[..count]);
                        }
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                        ) => {}
                    Err(e) => return Err(Failure::Input(e.to_string())),
                }
            }
            match child.try_wait() {
                Ok(Some(status)) if stdout_eof && stderr_eof => {
                    return if status.success() {
                        Ok(())
                    } else {
                        Err(Failure::Exit(status.code()))
                    };
                }
                Ok(_) => (),
                Err(e) => return Err(Failure::Input(e.to_string())),
            }
            if !progressed {
                thread::sleep(Duration::from_millis(2));
            }
        }
    })();
    // Kill the dedicated process group. Deliberate session escapes require an OS
    // sandbox/cgroup, outside this trusted-tool primitive's containment promise.
    // SAFETY: child was started in a new group with its positive PID as PGID.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.wait();
    result?;
    Ok(output)
}
#[cfg(not(unix))]
pub fn run(
    _: &Path,
    _: &[OsString],
    _: &Path,
    _: &BTreeMap<OsString, OsString>,
    _: &Budget,
) -> Result<Vec<u8>, Failure> {
    Err(Failure::UnsupportedPlatform)
}

static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct IsolatedProject {
    root: PathBuf,
    base: PathBuf,
}
impl IsolatedProject {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn tool_home(&self) -> PathBuf {
        self.base.join("tool-home")
    }
    pub fn copy(
        source: &Path,
        allowed_roots: &[PathBuf],
        budget: &Budget,
    ) -> Result<Self, Failure> {
        let source = source
            .canonicalize()
            .map_err(|e| Failure::Input(e.to_string()))?;
        if !allowed_roots
            .iter()
            .any(|r| r.canonicalize().is_ok_and(|r| source.starts_with(r)))
        {
            return Err(Failure::Input("project outside allowed roots".into()));
        }
        let root = std::env::temp_dir().join(format!(
            "archguard-isolated-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).map_err(|e| Failure::Input(e.to_string()))?;
        let base = root;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&base, fs::Permissions::from_mode(0o700))
                .map_err(|e| Failure::Input(e.to_string()))?;
        }
        let result = Self {
            root: base.join("project"),
            base,
        };
        for ancestor in result.base.ancestors() {
            for file in [".cargo/config", ".cargo/config.toml"] {
                match fs::symlink_metadata(ancestor.join(file)) {
                    Ok(_) => {
                        return Err(Failure::Input(
                            "ancestor Cargo configuration is unsupported".into(),
                        ));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(Failure::Input(e.to_string())),
                }
            }
        }
        fs::create_dir(result.root()).map_err(|e| Failure::Input(e.to_string()))?;
        fs::create_dir(result.tool_home()).map_err(|e| Failure::Input(e.to_string()))?;
        let mut state = CopyState {
            budget,
            bytes: 0,
            files: 0,
        };
        state.copy(&source, result.root(), 0)?;
        validate_manifests(result.root(), result.root())?;
        Ok(result)
    }
}
impl Drop for IsolatedProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}
struct CopyState<'a> {
    budget: &'a Budget,
    bytes: u64,
    files: usize,
}
impl CopyState<'_> {
    fn copy(&mut self, from: &Path, to: &Path, depth: usize) -> Result<(), Failure> {
        if depth > self.budget.depth {
            return Err(Failure::Input("input depth budget exceeded".into()));
        }
        for entry in fs::read_dir(from).map_err(|e| Failure::Input(e.to_string()))? {
            let entry = entry.map_err(|e| Failure::Input(e.to_string()))?;
            if entry.file_name() == ".git" || entry.file_name() == "target" {
                continue;
            }
            self.files += 1;
            if self.files > self.budget.files {
                return Err(Failure::Input("input entry budget exceeded".into()));
            }
            let ty = entry
                .file_type()
                .map_err(|e| Failure::Input(e.to_string()))?;
            let dest = to.join(entry.file_name());
            if ty.is_dir() {
                fs::create_dir(&dest).map_err(|e| Failure::Input(e.to_string()))?;
                self.copy(&entry.path(), &dest, depth + 1)?;
            } else if ty.is_file() {
                // O_NOFOLLOW prevents a final-component symlink swap during copy.
                let mut options = fs::OpenOptions::new();
                options.read(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.custom_flags(libc::O_NOFOLLOW);
                }
                let file = options
                    .open(entry.path())
                    .map_err(|e| Failure::Input(e.to_string()))?;
                let mut bytes = Vec::new();
                file.take(self.budget.file_bytes + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| Failure::Input(e.to_string()))?;
                self.bytes = self.bytes.saturating_add(bytes.len() as u64);
                if bytes.len() as u64 > self.budget.file_bytes
                    || self.bytes > self.budget.total_bytes
                {
                    return Err(Failure::Input("input byte budget exceeded".into()));
                }
                fs::write(dest, bytes).map_err(|e| Failure::Input(e.to_string()))?;
            } else {
                return Err(Failure::Input(
                    "symlink or special input file is unsupported".into(),
                ));
            }
        }
        Ok(())
    }
}
fn validate_manifests(root: &Path, dir: &Path) -> Result<(), Failure> {
    for entry in fs::read_dir(dir).map_err(|e| Failure::Input(e.to_string()))? {
        let entry = entry.map_err(|e| Failure::Input(e.to_string()))?;
        if entry
            .file_type()
            .map_err(|e| Failure::Input(e.to_string()))?
            .is_dir()
        {
            validate_manifests(root, &entry.path())?;
        } else if entry.file_name() == "Cargo.toml" {
            let bytes =
                fs::read_to_string(entry.path()).map_err(|e| Failure::Input(e.to_string()))?;
            let value: toml::Value = toml::from_str(&bytes)
                .map_err(|e| Failure::Input(format!("manifest syntax: {e}")))?;
            validate_paths(root, dir, "", &value)?;
        } else if dir.file_name().is_some_and(|n| n == ".cargo")
            && matches!(entry.file_name().to_str(), Some("config" | "config.toml"))
        {
            return Err(Failure::Input(
                "Cargo configuration is unsupported by isolated declaration profile".into(),
            ));
        }
    }
    Ok(())
}
fn validate_paths(root: &Path, dir: &Path, key: &str, value: &toml::Value) -> Result<(), Failure> {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                validate_paths(root, dir, key, value)?;
            }
        }
        toml::Value::Array(values) => {
            for value in values {
                validate_paths(root, dir, key, value)?;
            }
        }
        toml::Value::String(path)
            if matches!(
                key,
                "path"
                    | "workspace"
                    | "members"
                    | "exclude"
                    | "default-members"
                    | "readme"
                    | "license-file"
                    | "build"
            ) =>
        {
            let mut resolved = dir.to_path_buf();
            for part in Path::new(path).components() {
                match part {
                    Component::Normal(p) => resolved.push(p),
                    Component::CurDir => (),
                    Component::ParentDir => {
                        resolved.pop();
                    }
                    _ => {
                        return Err(Failure::Input(
                            "absolute manifest path is unsupported".into(),
                        ));
                    }
                }
                if !resolved.starts_with(root) {
                    return Err(Failure::Input("manifest path escapes isolated root".into()));
                }
            }
        }
        _ => (),
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Toolchain {
    pub cargo: PathBuf,
    pub rustc: PathBuf,
}
impl Toolchain {
    pub fn discover(budget: &Budget) -> Result<Self, Failure> {
        fn find(name: &str) -> Result<PathBuf, Failure> {
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .map(|dir| dir.join(name))
                .find(|path| path.is_file())
                .ok_or_else(|| Failure::Input(format!("missing tool: {name}")))
        }
        let environment: BTreeMap<OsString, OsString> = [
            "PATH",
            "HOME",
            "RUSTUP_HOME",
            "RUSTUP_TOOLCHAIN",
            "CARGO_HOME",
        ]
        .into_iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value)))
        .collect();
        let resolve = |name: &str| -> Result<PathBuf, Failure> {
            let path = if let Ok(rustup) = find("rustup") {
                let bytes = run(
                    &rustup,
                    &["which".into(), name.into()],
                    Path::new("/"),
                    &environment,
                    budget,
                )?;
                PathBuf::from(
                    String::from_utf8(bytes)
                        .map_err(|e| Failure::Input(e.to_string()))?
                        .trim(),
                )
            } else {
                find(name)?
            };
            path.canonicalize()
                .map_err(|e| Failure::Input(e.to_string()))
        };
        Ok(Self {
            cargo: resolve("cargo")?,
            rustc: resolve("rustc")?,
        })
    }
    pub fn environment(&self, home: &Path) -> BTreeMap<OsString, OsString> {
        [
            ("HOME".into(), home.as_os_str().into()),
            ("CARGO_HOME".into(), home.as_os_str().into()),
            ("RUSTC".into(), self.rustc.as_os_str().into()),
            (
                "PATH".into(),
                self.rustc
                    .parent()
                    .unwrap_or(Path::new("/"))
                    .as_os_str()
                    .into(),
            ),
        ]
        .into()
    }
}
