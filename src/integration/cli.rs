use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_OUTPUT: AtomicU64 = AtomicU64::new(0);

/// Clear explicitly requested destinations before analysis so failed retries cannot
/// leave a previous run's success at the same output path. Never follows symlinks.
pub fn invalidate_output(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Publish a complete file with same-directory rename. A failed write leaves no
/// truncated success artifact; callers must invalidate previous attempts first.
pub fn publish_output(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = None;
    for _ in 0..32 {
        let candidate = parent.join(format!(
            ".archguard-output-{}-{}",
            std::process::id(),
            NEXT_OUTPUT.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    let (temporary, mut file) = temporary.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "cannot allocate output temporary",
        )
    })?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn resolved_destination(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        return path.canonicalize();
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(parent.canonicalize()?.join(
        path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "output filename required")
        })?,
    ))
}

/// Invalidate every independently safe destination, even if another fails.
/// Existing project files are inputs, including previous reports placed there.
/// This is conservative alias validation over a stable tree, not an OS sandbox.
pub fn prepare_outputs(project: &Path, contract: &Path, paths: &[&Path]) -> io::Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut protected = ProtectedInputs::default();
    let protection = protected
        .collect_project(project)
        .and_then(|()| protected.collect(contract, 0));
    let mut errors = Vec::new();
    let mut resolved_outputs = std::collections::HashSet::new();
    let mut safe = Vec::new();
    for path in paths {
        let resolved = match resolved_destination(path) {
            Ok(path) => path,
            Err(error) => {
                errors.push(format!(
                    "{}: {error}; not modifying this path",
                    path.display()
                ));
                continue;
            }
        };
        if !resolved_outputs.insert(resolved.clone()) {
            errors.push("output aliases another output".into());
        }
        let existing = match fs::symlink_metadata(path) {
            Ok(_) => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => {
                errors.push(format!(
                    "{}: {error}; not modifying this path",
                    path.display()
                ));
                continue;
            }
        };
        if existing {
            if let Err(error) = &protection {
                errors.push(format!(
                    "cannot establish input aliases: {error}; not modifying {}",
                    path.display()
                ));
                continue;
            }
            let metadata = fs::metadata(path);
            let aliases = protected.paths.contains(&resolved)
                || metadata.as_ref().is_ok_and(|m| protected.same_file(m));
            if aliases {
                errors.push(format!(
                    "output aliases a protected project or contract input; not modifying {}",
                    path.display()
                ));
                continue;
            }
        }
        safe.push(*path);
    }
    for path in safe {
        if let Err(error) = invalidate_output(path) {
            errors.push(format!(
                "{}: {error}; could not invalidate this output",
                path.display()
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            errors.join("; "),
        ))
    }
}

#[derive(Default)]
struct ProtectedInputs {
    paths: std::collections::HashSet<PathBuf>,
    #[cfg(unix)]
    identities: std::collections::HashSet<(u64, u64)>,
    entries: usize,
}
impl ProtectedInputs {
    fn collect_project(&mut self, project: &Path) -> io::Result<()> {
        self.collect(project, 0)?;
        let root = match project.canonicalize() {
            Ok(root) => root,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        // Cargo can observe the entire workspace even when --project selects one
        // member. Inspect ancestry without running Cargo or writing lockfiles.
        for ancestor in root.ancestors() {
            let manifest = ancestor.join("Cargo.toml");
            let metadata = match fs::metadata(&manifest) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            self.collect(&manifest, 0)?;
            if metadata.len() > 16 * 1024 * 1024 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "workspace manifest inspection budget exceeded",
                ));
            }
            let bytes = fs::read(&manifest)?;
            let parsed = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|text| toml::from_str::<toml::Value>(text).ok());
            let Some(parsed) = parsed else {
                // A malformed ancestor cannot establish narrower workspace scope.
                return self.collect(ancestor, 0);
            };
            if let Some(workspace) = parsed
                .get("package")
                .and_then(|package| package.get("workspace"))
                .and_then(toml::Value::as_str)
            {
                return self.collect(&ancestor.join(workspace), 0);
            }
            if parsed.get("workspace").is_some() {
                return self.collect(ancestor, 0);
            }
        }
        Ok(())
    }

    fn same_file(&self, metadata: &fs::Metadata) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            self.identities.contains(&(metadata.dev(), metadata.ino()))
        }
        // Without stable file identity support, do not overwrite existing files.
        #[cfg(not(unix))]
        {
            let _ = metadata;
            true
        }
    }
    fn collect(&mut self, path: &Path, depth: usize) -> io::Result<()> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // A dangling source symlink is still an existing project entry.
                if fs::symlink_metadata(path).is_ok() {
                    self.paths.insert(resolved_destination(path)?);
                }
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let canonical = path.canonicalize()?;
        if !self.paths.insert(canonical.clone()) {
            return Ok(());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            self.identities.insert((metadata.dev(), metadata.ino()));
        }
        self.entries += 1;
        if self.entries > 100_000 || depth > 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "input alias inspection budget exceeded",
            ));
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                self.collect(&entry?.path(), depth + 1)?;
            }
        }
        Ok(())
    }
}
