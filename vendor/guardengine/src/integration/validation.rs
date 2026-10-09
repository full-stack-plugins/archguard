use super::*;
use crate::Decision;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
fn hex(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn digest(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(|s| hex(s, 64))
}
fn sorted_set(v: &[String]) -> bool {
    v.windows(2).all(|w| w[0] < w[1])
}
fn utc(s: &str) -> Result<OffsetDateTime, IntegrationError> {
    let t = OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|_| IntegrationError("invalid RFC3339 timestamp".into()))?;
    if !t.offset().is_utc() {
        return fail("timestamps must be UTC");
    }
    Ok(t)
}
fn bounds(v: &serde_json::Value, depth: usize) -> Result<(), IntegrationError> {
    if depth > 32 {
        return fail("maximum nesting depth exceeded");
    }
    match v {
        serde_json::Value::String(s) if s.trim().is_empty() || s.len() > 4096 => {
            return fail("blank or oversized string");
        }
        serde_json::Value::Array(a) => {
            if a.len() > 4096 {
                return fail("maximum collection length exceeded");
            }
            for v in a {
                bounds(v, depth + 1)?;
            }
        }
        serde_json::Value::Object(o) => {
            for v in o.values() {
                bounds(v, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}
/// Borrowed binding checks shared by envelopes and standalone approval validation.
/// No serialization or copied input allocation is needed for admission.
pub(super) fn validate_binding(b: &RunBinding) -> Result<(), IntegrationError> {
    let valid_string = |s: &str| !s.trim().is_empty() && s.len() <= 4096;
    if ![&b.repo_id, &b.task_id, &b.worktree_id]
        .iter()
        .all(|s| valid_string(s))
        || b.merge_group_id.as_ref().is_some_and(|s| !valid_string(s))
        || b.requirement_ids.len() > 4096
        || b.requirement_ids.iter().any(|s| !valid_string(s))
    {
        return fail("blank or oversized binding string or collection");
    }
    if ![&b.candidate_oid, &b.base_oid]
        .iter()
        .all(|s| hex(s, 40) || hex(s, 64))
    {
        return fail("invalid full object ID");
    }
    if !digest(&b.source_snapshot_digest) || b.baseline_digest.as_ref().is_some_and(|s| !digest(s))
    {
        return fail("invalid binding digest");
    }
    if b.requirement_ids.is_empty() {
        return fail("requirements and required scopes cannot be empty");
    }
    if !sorted_set(&b.requirement_ids) {
        return fail("set arrays must be sorted and unique");
    }
    Ok(())
}
impl ArtifactRef {
    fn validate(&self) -> Result<(), IntegrationError> {
        let Some(path) = self.uri.strip_prefix("artifact://") else {
            return fail("unsupported artifact URI");
        };
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() < 2
            || parts
                .iter()
                .any(|p| p.is_empty() || *p == "." || *p == "..")
            || path
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || "%\\?#@:".contains(c))
        {
            return fail("unsafe artifact URI");
        }
        if !digest(&self.digest) {
            return fail("invalid artifact digest");
        }
        Ok(())
    }
}
impl GuardRunEnvelope {
    /// Structural and cross-field checks only. Trust and native scope qualification are external.
    pub fn validate(&self, profile: EvidenceProfile) -> Result<(), IntegrationError> {
        let value = serde_json::to_value(self).map_err(|e| IntegrationError(e.to_string()))?;
        bounds(&value, 0)?;
        if serde_json::to_vec(self)
            .map_err(|e| IntegrationError(e.to_string()))?
            .len()
            > MAX_ENVELOPE_BYTES
        {
            return fail("envelope size exceeded");
        }
        if self.api_version != INTEGRATION_VERSION || self.kind != "GuardRunEnvelope" {
            return fail("unsupported envelope version or kind");
        }
        let b = &self.binding;
        validate_binding(b)?;
        let c = &self.coverage;
        if b.requirement_ids.is_empty() || c.required_scopes.is_empty() {
            return fail("requirements and required scopes cannot be empty");
        }
        for v in [
            &b.requirement_ids,
            &c.required_scopes,
            &c.observed_scopes,
            &c.missing_scopes,
            &self.approval_refs,
        ] {
            if !sorted_set(v) {
                return fail("set arrays must be sorted and unique");
            }
        }
        let missing: Vec<_> = c
            .required_scopes
            .iter()
            .filter(|s| c.observed_scopes.binary_search(s).is_err())
            .cloned()
            .collect();
        if missing != c.missing_scopes
            || (c.status == CoverageStatus::Complete) != missing.is_empty()
        {
            return fail("coverage set or completeness mismatch");
        }
        let started = utc(&self.started_at)?;
        let finished = utc(&self.finished_at)?;
        if finished < started {
            return fail("finishedAt precedes startedAt");
        }
        if let Some(e) = &self.expires_at {
            if utc(e)? < finished {
                return fail("expiresAt precedes finishedAt");
            }
        }
        let a = &self.artifacts;
        for r in a
            .contract
            .iter()
            .chain(a.facts.iter())
            .chain(a.report.iter())
            .chain(a.domain.iter())
        {
            r.validate()?;
        }
        match self.run_status {
            RunStatus::Completed => {
                if self.decision.is_none() {
                    return fail("completed run requires decision");
                }
                if c.status == CoverageStatus::Partial && self.decision != Some(Decision::Block) {
                    return fail("partial run must BLOCK");
                }
                match profile {
                    EvidenceProfile::EngineBacked => {
                        if a.contract.is_none() || a.facts.is_none() || a.report.is_none() {
                            return fail("engine-backed run requires all engine artifacts");
                        }
                    }
                    EvidenceProfile::NativeOnly => {
                        if a.contract.is_some()
                            || a.facts.is_some()
                            || a.report.is_some()
                            || a.domain.is_empty()
                        {
                            return fail(
                                "native-only run requires domain evidence and no engine artifacts",
                            );
                        }
                    }
                }
            }
            RunStatus::Error | RunStatus::Cancelled => {
                if self.decision.is_some() || self.diagnostics.is_empty() || a.report.is_some() {
                    return fail("failed run requires null decision, diagnostics and no report");
                }
            }
        }
        Ok(())
    }
}
pub fn load_envelope_json(
    bytes: &[u8],
    profile: EvidenceProfile,
) -> Result<GuardRunEnvelope, IntegrationError> {
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return fail("envelope size exceeded");
    }
    // Bound nesting before parsing; account for quotes and escaped quotes.
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for &b in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 32 {
                        return fail("maximum nesting depth exceeded");
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    // Deriving directly into closed structs rejects both duplicate fields and unknown fields.
    let value: GuardRunEnvelope =
        serde_json::from_slice(bytes).map_err(|e| IntegrationError(e.to_string()))?;
    value.validate(profile)?;
    Ok(value)
}
