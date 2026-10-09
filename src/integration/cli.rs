use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
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
    if let Err(error) = &protection {
        errors.push(format!("cannot establish input aliases: {error}"));
    }
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
    manifests: std::collections::HashSet<(PathBuf, PathBuf)>,
}
impl ProtectedInputs {
    fn collect_project(&mut self, project: &Path) -> io::Result<()> {
        self.collect(project, 0)?;
        let root = match project.canonicalize() {
            Ok(root) => root,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        let Some(parsed) = self.discover_manifest(&root.join("Cargo.toml"), 0)? else {
            return Ok(());
        };
        if parsed.get("workspace").is_some()
            || parsed
                .get("package")
                .and_then(|p| p.get("workspace"))
                .is_some()
        {
            return Ok(());
        }
        // Cargo can observe an enclosing workspace when a member was selected.
        for ancestor in root.ancestors().skip(1) {
            if let Some(parsed) = self.discover_manifest(&ancestor.join("Cargo.toml"), 0)? {
                if parsed.get("workspace").is_some() {
                    break;
                }
            }
        }
        Ok(())
    }

    fn discover_manifest(
        &mut self,
        manifest: &Path,
        depth: usize,
    ) -> io::Result<Option<toml::Value>> {
        if depth > 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "manifest reference depth budget exceeded",
            ));
        }
        let canonical = match manifest.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let source_root = manifest
            .parent()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "manifest has no source parent")
            })?
            .canonicalize()?;
        if !self
            .manifests
            .insert((canonical.clone(), source_root.clone()))
        {
            return Ok(None);
        }
        let root = canonical
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "manifest has no parent"))?;
        self.collect(root, depth)?;
        let mut bytes = Vec::new();
        fs::File::open(&canonical)?
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "manifest inspection budget exceeded",
            ));
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "manifest is not UTF-8"))?;
        let parsed: toml::Value = toml::from_str(text).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot establish references from invalid manifest",
            )
        })?;
        self.collect_references(root, &parsed, depth + 1)?;
        if source_root != root {
            // Cargo resolves a symlinked manifest's declarations from its source
            // location. Preserve both origins rather than narrowing ambiguously.
            self.collect(&source_root, depth)?;
            self.collect_references(&source_root, &parsed, depth + 1)?;
        }
        Ok(Some(parsed))
    }

    fn collect_references(
        &mut self,
        root: &Path,
        value: &toml::Value,
        depth: usize,
    ) -> io::Result<()> {
        if depth > 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "manifest reference depth budget exceeded",
            ));
        }
        match value {
            toml::Value::Table(table) => {
                for (key, value) in table {
                    if matches!(key.as_str(), "members" | "default-members") {
                        let members = value.as_array().ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "workspace member list must be an array",
                            )
                        })?;
                        for member in members {
                            let member = member.as_str().ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "workspace member must be a string",
                                )
                            })?;
                            let literal_root = root.to_str().ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "non UTF-8 workspace root",
                                )
                            })?;
                            // The filesystem root is literal; only the declaration
                            // supplies glob syntax. An absolute member replaces this
                            // prefix through Path::join and retains its own pattern.
                            let escaped_root = glob::Pattern::escape(literal_root);
                            let pattern = Path::new(&escaped_root).join(member);
                            let pattern = pattern.to_str().ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "non UTF-8 workspace member pattern",
                                )
                            })?;
                            let matches = glob::glob(pattern).map_err(|_| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "unsupported workspace member glob",
                                )
                            })?;
                            let mut found_member = false;
                            for (index, matched) in matches.enumerate() {
                                found_member = true;
                                if index >= 100_000 {
                                    return Err(io::Error::new(
                                        io::ErrorKind::InvalidInput,
                                        "workspace member match budget exceeded",
                                    ));
                                }
                                let member =
                                    matched.map_err(|error| io::Error::other(error.to_string()))?;
                                self.collect(&member, depth)?;
                                let manifest = member.join("Cargo.toml");
                                if !manifest.is_file() {
                                    return Err(io::Error::new(
                                        io::ErrorKind::InvalidInput,
                                        "declared workspace member has no readable manifest",
                                    ));
                                }
                                self.discover_manifest(&manifest, depth + 1)?;
                            }
                            if !found_member {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "declared workspace member pattern matched no input",
                                ));
                            }
                        }
                    } else if matches!(
                        key.as_str(),
                        "path" | "workspace" | "readme" | "license-file" | "build"
                    ) && value.is_str()
                    {
                        // This deliberately protects a superset: source target paths,
                        // package.workspace, dependency/patch paths and metadata paths.
                        let target = root.join(value.as_str().expect("checked string"));
                        self.collect(&target, depth)?;
                        if target.is_dir() {
                            self.discover_manifest(&target.join("Cargo.toml"), depth + 1)?;
                        }
                    } else {
                        self.collect_references(root, value, depth + 1)?;
                    }
                }
            }
            toml::Value::Array(values) => {
                for value in values {
                    self.collect_references(root, value, depth + 1)?;
                }
            }
            _ => (),
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
