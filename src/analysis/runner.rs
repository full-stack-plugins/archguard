//! Bounded fixed-tool Linux execution and isolated-input helpers; not a general OS sandbox.
//! The declaration entrypoint accepts only fixed Cargo/rustc tools and rejects
//! configurations that could select extra executables or external input paths.
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
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
impl Budget {
    fn validate(&self) -> Result<(), Failure> {
        let max = Self::default();
        if self.timeout.is_zero()
            || self.timeout > max.timeout
            || self.output_bytes == 0
            || self.output_bytes > max.output_bytes
            || self.file_bytes == 0
            || self.file_bytes > max.file_bytes
            || self.total_bytes == 0
            || self.total_bytes > max.total_bytes
            || self.files == 0
            || self.files > max.files
            || self.depth > max.depth
        {
            return Err(Failure::Input("invalid or excessive runner budget".into()));
        }
        Ok(())
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
    Cleanup,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Failure {}

#[cfg(all(
    target_os = "linux",
    target_endian = "little",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux;

#[cfg(all(
    target_os = "linux",
    target_endian = "little",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
/// Caller supplies an already trusted executable, arguments and environment.
/// This primitive grants no authority to arbitrary candidate-selected tools.
pub fn run(
    tool: &Path,
    args: &[OsString],
    cwd: &Path,
    environment: &BTreeMap<OsString, OsString>,
    budget: &Budget,
) -> Result<Vec<u8>, Failure> {
    use std::os::unix::process::CommandExt;
    budget.validate()?;
    let filter = linux::filter();
    let mut command = Command::new(tool);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    // SAFETY: precomputed filter; the hook only executes async-signal-safe
    // syscalls, after std establishes the initial group and before exec.
    unsafe {
        command.pre_exec(move || linux::install(&filter));
    }
    let mut child = command.spawn().map_err(|e| Failure::Start(e.to_string()))?;
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
            match linux::exited(child.id()) {
                Ok(true) if stdout_eof && stderr_eof => return Ok(()),
                Ok(_) => (),
                Err(e) => return Err(Failure::Input(e.to_string())),
            }
            if !progressed {
                thread::sleep(Duration::from_millis(2));
            }
        }
    })();
    // Leader has not been reaped: its numeric PID/PGID cannot be reused.
    // The inherited filter prevents descendants changing session or group.
    // SAFETY: child was started in a new group with its positive PID as PGID.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let cleanup = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if cleanup.elapsed() < Duration::from_millis(250) => {
                thread::sleep(Duration::from_millis(2))
            }
            _ => return Err(Failure::Cleanup),
        }
    };
    result?;
    if !status.success() {
        return Err(Failure::Exit(status.code()));
    }
    Ok(output)
}
#[cfg(not(all(
    target_os = "linux",
    target_endian = "little",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
pub fn run(
    _: &Path,
    _: &[OsString],
    _: &Path,
    _: &BTreeMap<OsString, OsString>,
    _: &Budget,
) -> Result<Vec<u8>, Failure> {
    Err(Failure::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
mod copy;
#[cfg(target_os = "linux")]
use copy::tree as copy_tree;
#[cfg(not(target_os = "linux"))]
fn copy_tree(_: &Path, _: &Path, _: &Budget, _: Instant) -> Result<(), Failure> {
    Err(Failure::UnsupportedPlatform)
}
pub struct IsolatedProject {
    root: PathBuf,
    base: tempfile::TempDir,
}
impl IsolatedProject {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn tool_home(&self) -> PathBuf {
        self.base.path().join("tool-home")
    }
    pub fn copy(
        source: &Path,
        allowed_roots: &[PathBuf],
        budget: &Budget,
    ) -> Result<Self, Failure> {
        budget.validate()?;
        let started = Instant::now();
        let source = source
            .canonicalize()
            .map_err(|e| Failure::Input(e.to_string()))?;
        if !allowed_roots
            .iter()
            .any(|r| r.canonicalize().is_ok_and(|r| source.starts_with(r)))
        {
            return Err(Failure::Input("project outside allowed roots".into()));
        }
        if started.elapsed() >= budget.timeout {
            return Err(Failure::Timeout);
        }
        let base = tempfile::Builder::new()
            .prefix("archguard-isolated-")
            .tempdir()
            .map_err(|e| Failure::Input(e.to_string()))?;
        let result = Self {
            root: base.path().join("project"),
            base,
        };
        for ancestor in result.base.path().ancestors() {
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
        copy_tree(&source, result.root(), budget, started)?;
        validate_manifests(result.root(), result.root())?;
        if started.elapsed() >= budget.timeout {
            return Err(Failure::Timeout);
        }
        Ok(result)
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
