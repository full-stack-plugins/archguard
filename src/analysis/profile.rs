use guardengine::{GuardAssertion, GuardContract};
use std::collections::BTreeSet;

/// Owned copy of obligations supplied by the caller's protected contract, never
/// discovered from candidate configuration. Authority is the caller's concern.
#[derive(Clone, Debug)]
pub struct FrozenAnalysisProfile {
    required_members: BTreeSet<String>,
    required_relations: BTreeSet<String>,
}

impl FrozenAnalysisProfile {
    pub fn freeze(
        contract: &GuardContract,
        required_members: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut members: BTreeSet<String> = required_members.into_iter().collect();
        let mut relations = BTreeSet::new();
        for rule in &contract.spec.rules {
            let GuardAssertion::ForbidRelation {
                subject,
                predicate,
                object,
            } = &rule.assertion;
            members.extend([subject.clone(), object.clone()]);
            relations.insert(predicate.clone());
        }
        Self {
            required_members: members,
            required_relations: relations,
        }
    }

    pub(crate) fn identity_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&(
            "cargo-declarations-v1",
            &self.required_members,
            &self.required_relations,
        ))
        .expect("string serialization is infallible")
    }

    pub(crate) fn members(&self) -> &BTreeSet<String> {
        &self.required_members
    }
    pub(crate) fn relations(&self) -> &BTreeSet<String> {
        &self.required_relations
    }

    pub fn coverage_gaps(&self, observed: &BTreeSet<String>) -> Vec<String> {
        let mut gaps: Vec<_> = self
            .required_members
            .difference(observed)
            .map(|name| format!("missing required member: {name}"))
            .collect();
        gaps.extend(
            self.required_relations
                .iter()
                .filter(|r| r.as_str() != "depends_on")
                .map(|relation| format!("unsupported relation: {relation}")),
        );
        gaps
    }
}
