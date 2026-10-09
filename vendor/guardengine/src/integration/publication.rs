//! Immutable per-attempt publication into a caller-protected private local directory.
//! Publication stores untrusted evidence; it does not make it current or eligible.
use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Seek},
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
pub struct StagedAttempt {
    file: Option<NamedTempFile>,
    destination: PathBuf,
    root: PathBuf,
    root_identity: (u64, u64),
    content_digest: String,
}
pub struct PublishedAttempt {
    pub path: PathBuf,
    pub content_digest: String,
}
fn io_error(e: std::io::Error) -> IntegrationError {
    IntegrationError(format!("publication I/O: {}", e.kind()))
}
fn private_root(root: &Path) -> Result<(u64, u64), IntegrationError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(root).map_err(io_error)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() || metadata.mode() & 0o077 != 0 {
            return fail("publication root must be a private non-symlink directory");
        }
        Ok((metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        let _ = root;
        fail("publication platform not qualified")
    }
}
/// Stage bounded canonical bytes. Dropping this value never publishes a final attempt.
/// Caller must keep this storage root inaccessible to untrusted writers throughout its lifetime.
pub fn stage_attempt(
    root: &Path,
    envelope: &GuardRunEnvelope,
    profile: EvidenceProfile,
) -> Result<StagedAttempt, IntegrationError> {
    stage_with_encoder(root, envelope, profile, |envelope, file| {
        serde_json::to_writer(file.as_file_mut(), envelope)
            .map_err(|e| IntegrationError(format!("publication encoding: {e}")))
    })
}
// Private seam: fault tests can exercise real encoder sink I/O errors without adding a public knob.
fn stage_with_encoder(
    root: &Path,
    envelope: &GuardRunEnvelope,
    profile: EvidenceProfile,
    encode: impl FnOnce(&GuardRunEnvelope, &mut NamedTempFile) -> Result<(), IntegrationError>,
) -> Result<StagedAttempt, IntegrationError> {
    envelope.validate(profile)?;
    let identity = private_root(root)?;
    let root = fs::canonicalize(root).map_err(io_error)?;
    if private_root(&root)? != identity {
        return fail("publication root changed");
    }
    let destination = root.join(format!(
        "attempt-{:x}.json",
        Sha256::digest(envelope.run_id.as_bytes())
    ));
    let mut file = tempfile::Builder::new()
        .prefix(".guard-attempt-")
        .tempfile_in(&root)
        .map_err(io_error)?;
    encode(envelope, &mut file)?;
    file.as_file().sync_all().map_err(io_error)?;
    // Hash the staged descriptor's actual bytes, not a separate serialization or pathname lookup.
    file.rewind().map_err(io_error)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(io_error)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(StagedAttempt {
        file: Some(file),
        destination,
        root,
        root_identity: identity,
        content_digest: format!("sha256:{:x}", hasher.finalize()),
    })
}
impl StagedAttempt {
    pub fn destination(&self) -> &Path {
        &self.destination
    }
    /// Never overwrites any existing final path, including a symlink or another attempt.
    /// A final path may exist if directory synchronization fails: reconcile its digest before retrying.
    pub fn publish(mut self) -> Result<PublishedAttempt, IntegrationError> {
        if private_root(&self.root)? != self.root_identity {
            return fail("publication root changed before commit");
        }
        let _file = self
            .file
            .take()
            .ok_or_else(|| IntegrationError("publication already consumed".into()))?
            .persist_noclobber(&self.destination)
            .map_err(|e| io_error(e.error))?;
        fs::File::open(&self.root).and_then(|dir|dir.sync_all()).map_err(|_|IntegrationError("attempt linked but directory synchronization failed; reconcile final bytes before retry".into()))?;
        Ok(PublishedAttempt {
            path: self.destination.clone(),
            content_digest: self.content_digest.clone(),
        })
    }
}

impl Drop for StagedAttempt {
    fn drop(&mut self) {
        // Never let pathname-based tempfile cleanup remove an entry in a replacement directory.
        // The orphan in the old store is safer than unlinking an unrelated replacement entry.
        if private_root(&self.root).ok() != Some(self.root_identity) {
            if let Some(file) = self.file.take() {
                let _ = file.keep();
            }
        }
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn serialization_sink_failure_removes_partial_stage_and_preserves_old_final() {
        let envelope: GuardRunEnvelope = serde_json::from_slice(include_bytes!(
            "../../tests/fixtures/integration-envelope/valid-native.json"
        ))
        .unwrap();
        let mut builder = tempfile::Builder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(fs::Permissions::from_mode(0o700));
        }
        let root = builder.tempdir().unwrap();
        let old = stage_attempt(root.path(), &envelope, EvidenceProfile::NativeOnly)
            .unwrap()
            .publish()
            .unwrap();
        let original = fs::read(&old.path).unwrap();
        let mut new_envelope = envelope;
        new_envelope.run_id = "encoder-failure".into();
        let mut prefix = Vec::new();
        let mut encoding_error = None;
        let result = stage_with_encoder(
            root.path(),
            &new_envelope,
            EvidenceProfile::NativeOnly,
            |value, file| {
                struct FailAfterPrefix<'a> {
                    writable: &'a mut fs::File,
                    read_only: fs::File,
                    remaining: usize,
                }
                impl Write for FailAfterPrefix<'_> {
                    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                        if self.remaining == 0 {
                            // Real OS EBADF/permission error, even when test runs as root.
                            return self.read_only.write(bytes);
                        }
                        let size = bytes.len().min(self.remaining);
                        let n = self.writable.write(&bytes[..size])?;
                        self.remaining -= n;
                        Ok(n)
                    }
                    fn flush(&mut self) -> std::io::Result<()> {
                        self.writable.flush()
                    }
                }
                let read_only = fs::File::open(file.path()).unwrap();
                let mut sink = FailAfterPrefix {
                    writable: file.as_file_mut(),
                    read_only,
                    remaining: 4,
                };
                let error = serde_json::to_writer(&mut sink, value).unwrap_err();
                assert!(
                    error.is_io(),
                    "typed envelope has no fabricated semantic serialization failure"
                );
                prefix = fs::read(file.path()).unwrap();
                let message = format!("publication encoding: {error}");
                encoding_error = Some(message.clone());
                Err(IntegrationError(message))
            },
        );
        assert!(
            result.is_err(),
            "a failed encoding sink must never return a publishable stage"
        );
        assert_eq!(
            prefix.len(),
            4,
            "actual partial bytes reached the temporary file"
        );
        assert_eq!(fs::read(&old.path).unwrap(), original);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        if let Some(capture_root) = std::env::var_os("GE_FAULT_CAPTURE_DIR") {
            let capture = Path::new(&capture_root).join("serialization-sink-failure");
            fs::create_dir_all(&capture).unwrap();
            fs::write(capture.join("partial-encoded.bin"), &prefix).unwrap();
            fs::write(capture.join("old-final.json"), &original).unwrap();
            fs::write(capture.join("error.txt"), encoding_error.unwrap()).unwrap();
            fs::write(capture.join("capture.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "fault_injection": "serde_json encoder writes four bytes to real stage then writes via read-only descriptor",
                "partial_bytes_written": prefix.len(), "io_error": true,
                "typed_semantic_serialization_failure_claimed": false,
                "new_stage_returned": false, "old_final_preserved": true,
                "partial_sha256": format!("{:x}", Sha256::digest(&prefix)),
                "old_final_sha256": format!("{:x}", Sha256::digest(&original))
            })).unwrap()).unwrap();
        }
    }
}
