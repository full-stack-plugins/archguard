//! GuardEngine: a deterministic core for rule, contract and evidence evaluation.
//! Domain-specific analyzers stay in individual Guard projects.
pub mod analyzer;
pub mod engine;
pub mod error;
pub mod protocol;

pub use analyzer::GuardAnalyzer;
pub use engine::{
    Decision, GuardReport, RuleEvaluation, RuleStatus, digest_json, evaluate, verify_report,
};
pub use error::GuardError;
pub use protocol::{
    API_VERSION, AnalyzerIdentity, Completeness, ContractMetadata, ContractSpec, Enforcement,
    GuardAssertion, GuardContract, GuardFact, GuardFacts, GuardRule, GuardSubject,
    load_contract_yaml, load_facts_json,
};

pub mod integration;
