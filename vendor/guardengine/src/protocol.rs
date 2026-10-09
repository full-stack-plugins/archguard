use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::error::GuardError;

pub const API_VERSION: &str = "guard.partme.ai/v1alpha1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardContract {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: ContractMetadata,
    pub spec: ContractSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractMetadata {
    pub id: String,
    pub revision: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractSpec {
    pub rules: Vec<GuardRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GuardRule {
    pub id: String,
    #[serde(default)]
    pub description: String,
    pub enforcement: Enforcement,
    pub assertion: GuardAssertion,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Enforcement {
    Enforce,
    Review,
    Advise,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum GuardAssertion {
    ForbidRelation {
        subject: String,
        predicate: String,
        object: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardFacts {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub analyzer: AnalyzerIdentity,
    pub subject: GuardSubject,
    pub completeness: Completeness,
    pub facts: Vec<GuardFact>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerIdentity {
    pub id: String,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardSubject {
    pub id: String,
    /// Hash of the exact source material actually analyzed, not necessarily a Git commit.
    pub snapshot_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Complete,
    Partial,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct GuardFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub source: String,
}

impl GuardContract {
    pub fn validate(&self) -> Result<(), GuardError> {
        if self.api_version != API_VERSION || self.kind != "GuardContract" {
            return Err(GuardError::InvalidProtocol(format!(
                "expected apiVersion={API_VERSION}, kind=GuardContract"
            )));
        }
        require_nonempty("metadata.id", &self.metadata.id)?;
        require_nonempty("metadata.revision", &self.metadata.revision)?;
        if self.spec.rules.is_empty() {
            return Err(GuardError::InvalidProtocol(
                "spec.rules cannot be empty".into(),
            ));
        }
        let mut ids = HashSet::new();
        for rule in &self.spec.rules {
            require_nonempty("rule.id", &rule.id)?;
            if !ids.insert(&rule.id) {
                return Err(GuardError::InvalidProtocol(format!(
                    "duplicate rule id: {}",
                    rule.id
                )));
            }
            match &rule.assertion {
                GuardAssertion::ForbidRelation {
                    subject,
                    predicate,
                    object,
                } => {
                    require_nonempty("assertion.subject", subject)?;
                    require_nonempty("assertion.predicate", predicate)?;
                    require_nonempty("assertion.object", object)?;
                }
            }
        }
        Ok(())
    }
}

impl GuardFacts {
    pub fn validate(&self) -> Result<(), GuardError> {
        if self.api_version != API_VERSION || self.kind != "GuardFacts" {
            return Err(GuardError::InvalidProtocol(format!(
                "expected apiVersion={API_VERSION}, kind=GuardFacts"
            )));
        }
        require_nonempty("analyzer.id", &self.analyzer.id)?;
        require_nonempty("analyzer.version", &self.analyzer.version)?;
        require_nonempty("subject.id", &self.subject.id)?;
        require_nonempty("subject.snapshotDigest", &self.subject.snapshot_digest)?;
        for fact in &self.facts {
            require_nonempty("fact.subject", &fact.subject)?;
            require_nonempty("fact.predicate", &fact.predicate)?;
            require_nonempty("fact.object", &fact.object)?;
            require_nonempty("fact.source", &fact.source)?;
        }
        if self.completeness == Completeness::Partial && self.diagnostics.is_empty() {
            return Err(GuardError::InvalidProtocol(
                "partial analyzer results must explain incompleteness in diagnostics".into(),
            ));
        }
        Ok(())
    }
}

fn require_nonempty(name: &str, value: &str) -> Result<(), GuardError> {
    if value.trim().is_empty() {
        Err(GuardError::InvalidProtocol(format!(
            "{name} cannot be blank"
        )))
    } else {
        Ok(())
    }
}

/// Contract metadata is YAML for human review, facts are JSON for analyzer interchange.
/// No arbitrary code or expression evaluation is performed during parsing.
pub fn load_contract_yaml(bytes: &[u8]) -> Result<GuardContract, GuardError> {
    let contract: GuardContract = serde_yaml::from_slice(bytes)
        .map_err(|error| GuardError::Input(format!("invalid contract YAML: {error}")))?;
    contract.validate()?;
    Ok(contract)
}

pub fn load_facts_json(bytes: &[u8]) -> Result<GuardFacts, GuardError> {
    let facts: GuardFacts = serde_json::from_slice(bytes)
        .map_err(|error| GuardError::Input(format!("invalid facts JSON: {error}")))?;
    facts.validate()?;
    Ok(facts)
}
