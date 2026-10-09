use std::path::Path;

use crate::{GuardError, GuardFacts};

/// Standard language-analysis adapter contract. Extraction is separate from policy evaluation.
/// Each analyzer must record what it actually inspected in subject.snapshot_digest.
pub trait GuardAnalyzer {
    fn analyze(&self, project_root: &Path, subject_id: &str) -> Result<GuardFacts, GuardError>;
}
