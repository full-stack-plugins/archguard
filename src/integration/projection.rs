//! Strict independent Cargo declaration artifacts. Integrity is not authentication.
use crate::analysis::{cargo::DependencyDeclaration, profile::FrozenAnalysisProfile};
use guardengine::{
    Completeness, GuardContract, GuardFacts,
    integration::{
        self, ArtifactRef, Coverage, CoverageStatus, EvidenceProfile, GuardRunEnvelope, RunStatus,
        TransportDiagnostic,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
pub const BUNDLE_VERSION: &str = "archguard.evidence/v1alpha1";
pub const DOMAIN_VERSION: &str = "archguard.cargo-domain/v1alpha1";
pub const PROFILE: &str = "cargo-declarations-v1";
pub(crate) fn diagnostic(code: &str, message: &str) -> TransportDiagnostic {
    TransportDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}
pub(crate) fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
/// Bound serialization refuses before growing an artifact beyond the shared byte cap.
pub(crate) fn bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    struct Sink(Vec<u8>);
    impl std::io::Write for Sink {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            if data.len() > integration::MAX_ARTIFACT_BYTES.saturating_sub(self.0.len()) {
                return Err(std::io::Error::other("artifact budget exceeded"));
            }
            self.0.extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut sink = Sink(vec![]);
    serde_json::to_writer(&mut sink, value)
        .map_err(|_| "artifact serialization budget exceeded")?;
    Ok(sink.0)
}
pub(crate) fn value<T: Serialize>(input: &T) -> Result<Value, String> {
    serde_json::from_slice(&bytes(input)?).map_err(|_| "invalid artifact".into())
}
pub(crate) fn reference(name: &str, value: &Value) -> Result<ArtifactRef, String> {
    let digest = digest(&bytes(value)?);
    Ok(ArtifactRef {
        uri: format!(
            "artifact://archguard/{name}/{}",
            digest.trim_start_matches("sha256:")
        ),
        digest,
        media_type: "application/json".into(),
    })
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateContext {
    pub repo_id: String,
    pub task_id: String,
    pub worktree_id: String,
    pub requirement_ids: Vec<String>,
    pub candidate_oid: String,
    pub base_oid: String,
    #[serde(deserialize_with = "required_nullable")]
    pub merge_group_id: Option<String>,
}
fn required_nullable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}
/// Owned, bounded policy supplied by a trusted controller; this type does not authenticate it.
pub struct ProtectedCargoPolicy {
    pub(crate) contract: GuardContract,
    pub(crate) profile: FrozenAnalysisProfile,
    pub(crate) profile_digest: String,
}
impl ProtectedCargoPolicy {
    pub fn freeze(
        contract: GuardContract,
        members: Vec<String>,
    ) -> Result<Self, TransportDiagnostic> {
        let bad = || {
            diagnostic(
                "policy.invalid",
                "unsupported or oversized Cargo evidence policy",
            )
        };
        // Inspect existing typed inputs before building member/relation sets or coverage.
        if contract.spec.rules.len() > 256
            || members.len() > 1024
            || members.iter().any(|s| s.is_empty() || s.len() > 256)
        {
            return Err(bad());
        }
        let raw = bytes(&contract).map_err(|_| bad())?;
        if raw.len() > 128 * 1024 {
            return Err(bad());
        }
        contract.validate().map_err(|_| bad())?;
        let profile = FrozenAnalysisProfile::freeze(&contract, members);
        if profile.members().len() > 1024
            || profile
                .members()
                .iter()
                .chain(profile.relations())
                .any(|s| s.is_empty() || s.len() > 256)
        {
            return Err(bad());
        }
        let profile_digest = digest(
            &bytes(&(PROFILE, &contract, profile.members(), profile.relations()))
                .map_err(|_| bad())?,
        );
        Ok(Self {
            contract,
            profile,
            profile_digest,
        })
    }
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }
    pub(crate) fn coverage(&self, observed: Option<&BTreeSet<String>>) -> Coverage {
        let mut required = BTreeSet::from([
            format!("cargo.profile:{}", self.profile_digest),
            "cargo.declarations".into(),
        ]);
        let mut actual = if observed.is_some() {
            required.clone()
        } else {
            BTreeSet::new()
        };
        for member in self.profile.members() {
            let scope = format!("cargo.member:{}", digest(member.as_bytes()));
            required.insert(scope.clone());
            if observed.is_some_and(|m| m.contains(member)) {
                actual.insert(scope);
            }
        }
        for relation in self.profile.relations() {
            let scope = format!("cargo.relation:{}", digest(relation.as_bytes()));
            required.insert(scope.clone());
            if observed.is_some() && relation == "depends_on" {
                actual.insert(scope);
            }
        }
        let missing: Vec<_> = required.difference(&actual).cloned().collect();
        Coverage {
            status: if missing.is_empty() {
                CoverageStatus::Complete
            } else {
                CoverageStatus::Partial
            },
            required_scopes: required.into_iter().collect(),
            observed_scopes: actual.into_iter().collect(),
            missing_scopes: missing,
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CargoDomain {
    pub api_version: String,
    pub profile: String,
    pub profile_digest: String,
    pub candidate_binding: String,
    pub advisory: bool,
    pub authority: String,
    pub source_snapshot_digest: String,
    pub manifest_digest: String,
    pub inventory_keys: Vec<String>,
    pub required_members: BTreeSet<String>,
    pub required_relations: BTreeSet<String>,
    pub observed_members: BTreeSet<String>,
    pub declarations: Vec<DependencyDeclaration>,
}
fn artifact_input<'de, D, T>(d: D) -> Result<Option<Value>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Serialize,
{
    let artifact = Option::<T>::deserialize(d)?;
    artifact
        .map(|item| value(&item).map_err(serde::de::Error::custom))
        .transpose()
}
fn contract_input<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Value>, D::Error> {
    artifact_input::<D, GuardContract>(d)
}
fn facts_input<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Value>, D::Error> {
    artifact_input::<D, GuardFacts>(d)
}
fn report_input<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Value>, D::Error> {
    artifact_input::<D, guardengine::GuardReport>(d)
}
fn domain_input<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Value>, D::Error> {
    artifact_input::<D, CargoDomain>(d)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBundle {
    pub api_version: String,
    pub envelope: GuardRunEnvelope,
    #[serde(deserialize_with = "contract_input")]
    pub contract: Option<Value>,
    #[serde(deserialize_with = "facts_input")]
    pub facts: Option<Value>,
    #[serde(deserialize_with = "report_input")]
    pub report: Option<Value>,
    #[serde(deserialize_with = "domain_input")]
    pub domain: Option<Value>,
}
impl EvidenceBundle {
    pub fn load(input: &[u8]) -> Result<Self, String> {
        if input.len() > 4 * integration::MAX_ARTIFACT_BYTES + 1024 * 1024 {
            return Err("bundle budget exceeded".into());
        }
        let bundle: Self =
            serde_json::from_slice(input).map_err(|_| "invalid strict evidence bundle")?;
        bundle.verify()?;
        Ok(bundle)
    }
    pub fn verify(&self) -> Result<(), String> {
        if self.api_version != BUNDLE_VERSION {
            return Err("unsupported evidence version".into());
        }
        self.envelope
            .validate(EvidenceProfile::EngineBacked)
            .map_err(|_| "invalid envelope")?;
        if self.envelope.producer.guard != "ArchGuard"
            || self.envelope.producer.analyzer_id != "archguard.cargo.declarations"
            || self.envelope.producer.analyzer_version != env!("CARGO_PKG_VERSION")
            || self.envelope.producer.version != env!("CARGO_PKG_VERSION")
        {
            return Err("unsupported producer profile".into());
        }
        if self.envelope.run_status != RunStatus::Completed {
            if self.contract.is_some()
                || self.facts.is_some()
                || self.report.is_some()
                || self.domain.is_some()
                || !self.envelope.artifacts.domain.is_empty()
                || !self.envelope.coverage.observed_scopes.is_empty()
            {
                return Err("failed attempt carries success artifacts or coverage".into());
            }
            return Ok(());
        }
        for (name, artifact, reference_value) in [
            (
                "contract",
                self.contract.as_ref(),
                self.envelope.artifacts.contract.as_ref(),
            ),
            (
                "facts",
                self.facts.as_ref(),
                self.envelope.artifacts.facts.as_ref(),
            ),
            (
                "report",
                self.report.as_ref(),
                self.envelope.artifacts.report.as_ref(),
            ),
        ] {
            if reference_value
                != Some(&reference(
                    name,
                    artifact.ok_or("missing engine artifact")?,
                )?)
            {
                return Err("unsupported engine artifact reference".into());
            }
        }
        let c = bytes(self.contract.as_ref().ok_or("missing contract")?)?;
        let f = bytes(self.facts.as_ref().ok_or("missing facts")?)?;
        let r = bytes(self.report.as_ref().ok_or("missing report")?)?;
        let domain_value = self.domain.as_ref().ok_or("missing domain")?;
        if self.envelope.artifacts.domain != vec![reference("domain", domain_value)?] {
            return Err("domain reference mismatch".into());
        }
        let domain: CargoDomain =
            serde_json::from_slice(&bytes(domain_value)?).map_err(|_| "invalid domain fields")?;
        if domain.api_version != DOMAIN_VERSION
            || domain.profile != PROFILE
            || !domain.advisory
            || domain.authority != "controller-resolved-unverified"
            || domain.source_snapshot_digest != self.envelope.binding.source_snapshot_digest
            || domain.candidate_binding != digest(&bytes(&self.envelope.binding)?)
        {
            return Err("unsupported domain or binding mismatch".into());
        }
        let valid_digest = |s: &str| {
            s.strip_prefix("sha256:").is_some_and(|h| {
                h.len() == 64
                    && h.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        };
        if !valid_digest(&domain.manifest_digest)
            || !domain.inventory_keys.windows(2).all(|w| w[0] < w[1])
            || ![
                "identity:profile",
                "identity:tool:cargo",
                "identity:tool:rustc",
            ]
            .iter()
            .all(|required| domain.inventory_keys.iter().any(|key| key == required))
            || domain
                .inventory_keys
                .iter()
                .any(|key| !(key.starts_with("file:") || key.starts_with("identity:")))
        {
            return Err("invalid declared inventory".into());
        }
        let contract = guardengine::load_contract_yaml(&c).map_err(|_| "invalid contract")?;
        let policy = ProtectedCargoPolicy::freeze(
            contract,
            domain.required_members.iter().cloned().collect(),
        )
        .map_err(|_| "invalid domain policy")?;
        if policy.profile_digest != domain.profile_digest
            || policy.profile.relations() != &domain.required_relations
            || policy.coverage(Some(&domain.observed_members)) != self.envelope.coverage
        {
            return Err("domain coverage mismatch".into());
        }
        integration::verify_engine_artifacts(&self.envelope, &c, &f, &r)
            .map_err(|_| "engine evidence mismatch")?;
        let facts: GuardFacts = guardengine::load_facts_json(&f).map_err(|_| "invalid facts")?;
        let mut projected = vec![];
        for d in domain.declarations {
            if !domain.observed_members.contains(&d.member) {
                return Err("unknown declaration member".into());
            }
            if let Some(target) = d.member_target {
                if !domain.observed_members.contains(&target) {
                    return Err("unknown declaration target".into());
                }
                projected.push(guardengine::GuardFact {
                    subject: d.member,
                    predicate: "depends_on".into(),
                    object: target,
                    source: d.source,
                });
            }
        }
        projected.sort();
        projected.dedup();
        if projected != facts.facts
            || (facts.completeness == Completeness::Complete)
                != (self.envelope.coverage.status == CoverageStatus::Complete)
        {
            return Err("declaration projection mismatch".into());
        }
        Ok(())
    }
}
pub use super::evidence::CargoEvidence;
