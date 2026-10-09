use super::*;
/// Verify byte integrity, native report recomputation and envelope/report agreement.
/// Successful verification is not producer authentication or Git object proof.
pub fn verify_engine_artifacts(
    envelope: &GuardRunEnvelope,
    contract_yaml: &[u8],
    facts_json: &[u8],
    report_json: &[u8],
) -> Result<crate::GuardReport, IntegrationError> {
    use sha2::{Digest, Sha256};
    envelope.validate(EvidenceProfile::EngineBacked)?;
    if envelope.run_status != RunStatus::Completed {
        return fail("only completed evidence can be verified");
    }
    for (reference, bytes) in [
        (&envelope.artifacts.contract, contract_yaml),
        (&envelope.artifacts.facts, facts_json),
        (&envelope.artifacts.report, report_json),
    ] {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return fail("artifact size exceeded");
        }
        let reference = reference
            .as_ref()
            .ok_or_else(|| IntegrationError("missing artifact reference".into()))?;
        if reference.digest != format!("sha256:{:x}", Sha256::digest(bytes)) {
            return fail("artifact byte digest mismatch");
        }
    }
    super::yaml_validation::validate_contract_yaml(contract_yaml)?;
    let contract =
        crate::load_contract_yaml(contract_yaml).map_err(|e| IntegrationError(e.to_string()))?;
    let facts = crate::load_facts_json(facts_json).map_err(|e| IntegrationError(e.to_string()))?;
    check_evaluation_budget(&contract, &facts, contract_yaml.len(), facts_json.len())?;
    let report: crate::GuardReport =
        serde_json::from_slice(report_json).map_err(|e| IntegrationError(e.to_string()))?;
    if !crate::verify_report(&report, &contract, &facts)
        .map_err(|e| IntegrationError(e.to_string()))?
    {
        return fail("report differs from recomputation");
    }
    if report.analyzer.id != envelope.producer.analyzer_id
        || report.analyzer.version != envelope.producer.analyzer_version
        || report.subject.snapshot_digest != envelope.binding.source_snapshot_digest
        || envelope.decision.as_ref() != Some(&report.decision)
    {
        return fail("envelope and engine report disagree");
    }
    if (facts.completeness == crate::Completeness::Complete)
        != (envelope.coverage.status == CoverageStatus::Complete)
    {
        return fail("envelope and analyzer completeness disagree");
    }
    Ok(report)
}
/// Bounded producer evaluation on the new integration surface. Native `evaluate` is unchanged.
pub fn evaluate_bounded(
    contract: &crate::GuardContract,
    facts: &crate::GuardFacts,
) -> Result<crate::GuardReport, IntegrationError> {
    let contract_size = bounded_json_size(contract)?;
    let facts_size = bounded_json_size(facts)?;
    check_evaluation_budget(contract, facts, contract_size, facts_size)?;
    crate::evaluate(contract, facts).map_err(|e| IntegrationError(e.to_string()))
}
fn check_evaluation_budget(
    contract: &crate::GuardContract,
    facts: &crate::GuardFacts,
    contract_size: usize,
    facts_size: usize,
) -> Result<(), IntegrationError> {
    // Core evaluation clones matching facts and joined partial diagnostics for every rule.
    let rules = contract.spec.rules.len();
    let per_rule = facts_size
        .saturating_add(
            facts
                .facts
                .len()
                .saturating_mul(std::mem::size_of::<crate::GuardFact>()),
        )
        .saturating_add(facts.diagnostics.len().saturating_mul(2))
        .saturating_add(std::mem::size_of::<crate::RuleEvaluation>())
        .saturating_add(128);
    if rules.saturating_mul(per_rule).saturating_add(contract_size) > MAX_ARTIFACT_BYTES
        || rules.saturating_mul(facts.facts.len()) > 1_000_000
    {
        return fail("recomputation budget exceeded");
    }
    Ok(())
}
pub(super) fn bounded_json_size<T: serde::Serialize>(value: &T) -> Result<usize, IntegrationError> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_ARTIFACT_BYTES.saturating_sub(self.0) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "artifact size exceeded",
                ));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value)
        .map_err(|_| IntegrationError("artifact size exceeded or serialization failed".into()))?;
    Ok(counter.0)
}
