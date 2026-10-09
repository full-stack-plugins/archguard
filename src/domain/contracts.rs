//! Approved local domain declarations evaluated against actual supported Java facts.
//! Static type access is not runtime state-transition or test-execution evidence.
use super::{baseline::ArchitectureBaseline, model::*};
use crate::{
    analysis::java::JavaAnalysis,
    integration::projection::{bytes, digest},
};
use guardengine::{
    integration::eligibility::{AuthorityProvider, EligibilityPolicy},
    *,
};
use serde::Serialize;
use std::collections::BTreeSet;
pub const DOMAIN_PROFILE: &str = "archguard.domain-static-access/v1alpha1";
#[derive(Clone, Serialize)]
pub struct Context {
    pub id: String,
    pub members: BTreeSet<SymbolId>,
}
#[derive(Clone, Serialize)]
pub struct Aggregate {
    pub id: String,
    pub context: String,
    pub entry_type: SymbolId,
    pub internal_state_type: SymbolId,
}
#[derive(Clone, Serialize)]
pub struct Transition {
    pub id: String,
    pub aggregate: String,
    pub method: SymbolId,
    pub from_state: String,
    pub to_state: String,
}
#[derive(Clone, Serialize)]
pub enum InvariantKind {
    StaticStateAccess,
    HeuristicStateAccess,
    TransitionEntry { transition: String },
}
#[derive(Clone, Serialize)]
pub struct Invariant {
    pub id: String,
    pub aggregate: String,
    pub enforcement: Enforcement,
    pub kind: InvariantKind,
}
fn admit(v: &impl Serialize) -> Result<(), String> {
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(b.len());
            if self.0 > 1024 * 1024 {
                return Err(std::io::Error::other("domain input budget"));
            }
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Count(0), v).map_err(|_| "domain input budget".into())
}
fn id(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 256 && !s.contains(['\0', '\n', '\r'])
}
fn unique<'a>(mut ids: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    ids.all(|v| id(v) && seen.insert(v))
}
/// Immutable protected specification. Baseline approval is freshly checked at evaluation.
#[derive(Serialize)]
pub struct DomainContract {
    provider: String,
    contexts: Vec<Context>,
    aggregates: Vec<Aggregate>,
    invariants: Vec<Invariant>,
    transitions: Vec<Transition>,
    contract: GuardContract,
    obligations: BTreeSet<String>,
    spec_digest: String,
}
impl DomainContract {
    pub fn freeze(
        provider: &str,
        mut contexts: Vec<Context>,
        mut aggregates: Vec<Aggregate>,
        mut invariants: Vec<Invariant>,
        mut transitions: Vec<Transition>,
    ) -> Result<Self, String> {
        if provider.is_empty()
            || provider.len() > 512
            || contexts.is_empty()
            || contexts.len() > 64
            || aggregates.is_empty()
            || aggregates.len() > 128
            || invariants.is_empty()
            || invariants.len() > 128
            || transitions.len() > 128
        {
            return Err("domain count/profile budget".into());
        }
        admit(&(provider, &contexts, &aggregates, &invariants, &transitions))?;
        if !unique(contexts.iter().map(|v| v.id.as_str()))
            || !unique(aggregates.iter().map(|v| v.id.as_str()))
            || !unique(invariants.iter().map(|v| v.id.as_str()))
            || !unique(transitions.iter().map(|v| v.id.as_str()))
        {
            return Err("duplicate/invalid domain identity".into());
        }
        let mut members = BTreeSet::new();
        for c in &contexts {
            if c.members.is_empty() || c.members.len() > 256 {
                return Err("empty/oversized context".into());
            }
            for s in &c.members {
                if s.language() != Language::Java || !members.insert(s) {
                    return Err("unsupported language or ambiguous context ownership".into());
                }
            }
        }
        for a in &aggregates {
            let c = contexts
                .iter()
                .find(|c| c.id == a.context)
                .ok_or("unknown aggregate context")?;
            if a.entry_type == a.internal_state_type
                || a.entry_type.kind() != SymbolKind::Type
                || a.internal_state_type.kind() != SymbolKind::Type
                || !c.members.contains(&a.entry_type)
                || !c.members.contains(&a.internal_state_type)
            {
                return Err("invalid aggregate type scope".into());
            }
        }
        for t in &transitions {
            let a = aggregates
                .iter()
                .find(|a| a.id == t.aggregate)
                .ok_or("unknown transition aggregate")?;
            let c = contexts.iter().find(|c| c.id == a.context).unwrap();
            if t.method.kind() != SymbolKind::Method
                || !c.members.contains(&t.method)
                || !id(&t.from_state)
                || !id(&t.to_state)
                || t.from_state == t.to_state
            {
                return Err("invalid explicit transition".into());
            }
        }
        for i in &invariants {
            if !aggregates.iter().any(|a| a.id == i.aggregate) {
                return Err("unknown invariant aggregate".into());
            }
            match &i.kind {
                InvariantKind::HeuristicStateAccess if i.enforcement == Enforcement::Enforce => {
                    return Err("heuristic cannot enforce".into());
                }
                InvariantKind::TransitionEntry { transition }
                    if !transitions
                        .iter()
                        .any(|t| t.id == *transition && t.aggregate == i.aggregate) =>
                {
                    return Err("unknown transition invariant".into());
                }
                _ => {}
            }
        }
        contexts.sort_by(|a, b| a.id.cmp(&b.id));
        aggregates.sort_by(|a, b| a.id.cmp(&b.id));
        invariants.sort_by(|a, b| a.id.cmp(&b.id));
        transitions.sort_by(|a, b| a.id.cmp(&b.id));
        let spec_digest = digest(&bytes(&(
            DOMAIN_PROFILE,
            provider,
            &contexts,
            &aggregates,
            &invariants,
            &transitions,
        ))?);
        let contract = GuardContract {
            api_version: API_VERSION.into(),
            kind: "GuardContract".into(),
            metadata: ContractMetadata {
                id: "archguard-domain".into(),
                revision: spec_digest.clone(),
            },
            spec: ContractSpec {
                rules: invariants
                    .iter()
                    .map(|i| GuardRule {
                        id: i.id.clone(),
                        description: "explicit approved domain rule".into(),
                        enforcement: i.enforcement.clone(),
                        assertion: GuardAssertion::ForbidRelation {
                            subject: i.id.clone(),
                            predicate: DOMAIN_PROFILE.into(),
                            object: spec_digest.clone(),
                        },
                    })
                    .collect(),
            },
        };
        contract.validate().map_err(|e| e.to_string())?;
        let obligations = invariants
            .iter()
            .map(|i| ("invariant", i.id.as_str()))
            .chain(transitions.iter().map(|t| ("transition", t.id.as_str())))
            .map(|(kind, id)| {
                format!(
                    "ag-test-obligation:{}",
                    digest(&bytes(&(&spec_digest, kind, id)).expect("bounded obligation identity"))
                )
            })
            .collect();
        Ok(Self {
            provider: provider.into(),
            contexts,
            aggregates,
            invariants,
            transitions,
            contract,
            obligations,
            spec_digest,
        })
    }
    pub fn contract(&self) -> &GuardContract {
        &self.contract
    }
    pub fn required_obligations(&self) -> &BTreeSet<String> {
        &self.obligations
    }
    pub fn digest(&self) -> &str {
        &self.spec_digest
    }
    /// Only an actual immutable native analysis is accepted; caller-built model completeness cannot fill a gap.
    pub fn evaluate_java(
        &self,
        analysis: &JavaAnalysis,
        baseline: &ArchitectureBaseline,
        policy: &EligibilityPolicy,
        authority: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<DomainEvaluation, String> {
        if baseline.contract_digest() != digest(&bytes(&self.contract)?)
            || baseline.obligations() != &self.obligations
        {
            return Err("baseline domain contract or obligation mismatch".into());
        }
        baseline.qualify(policy, authority, now)?;
        let o = analysis.observation();
        o.bounded_input_bytes().map_err(str::to_string)?;
        if o.provider_profile() != self.provider {
            return Err("foreign native provider profile".into());
        }
        let present = o.symbols().map(|s| &s.id).collect::<BTreeSet<_>>();
        let required = self
            .contexts
            .iter()
            .flat_map(|c| &c.members)
            .collect::<BTreeSet<_>>();
        let mut gaps = BTreeSet::new();
        if !required.is_subset(&present) {
            gaps.insert("missing required domain symbols".to_string());
        }
        if !o.is_complete(Relation::DependsOn) {
            gaps.insert("incomplete static dependency coverage".to_string());
        }
        let mut findings = Vec::new();
        let mut cost = 0usize;
        for i in &self.invariants {
            if matches!(i.kind, InvariantKind::TransitionEntry { .. }) {
                gaps.insert(format!(
                    "unsupported transition execution semantics: {}",
                    i.id
                ));
                continue;
            }
            let aggregate = self
                .aggregates
                .iter()
                .find(|a| a.id == i.aggregate)
                .unwrap();
            if let Some(edge) = o.edges().iter().find(|e| {
                e.relation == Relation::DependsOn
                    && e.to == aggregate.internal_state_type
                    && e.from != aggregate.entry_type
                    && e.from != aggregate.internal_state_type
            }) {
                cost = cost.saturating_add(edge.input_bytes());
                if cost > 1024 * 1024 {
                    return Err("domain witness budget".into());
                }
                findings.push(DomainFinding {
                    invariant: i.id.clone(),
                    witness: edge.clone(),
                });
            }
        }
        let facts = GuardFacts {
            api_version: API_VERSION.into(),
            kind: "GuardFacts".into(),
            analyzer: AnalyzerIdentity {
                id: DOMAIN_PROFILE.into(),
                version: "1".into(),
            },
            subject: GuardSubject {
                id: "local-domain-java-analysis".into(),
                snapshot_digest: analysis.source_digest().into(),
            },
            completeness: if gaps.is_empty() {
                Completeness::Complete
            } else {
                Completeness::Partial
            },
            facts: findings
                .iter()
                .map(|f| GuardFact {
                    subject: f.invariant.clone(),
                    predicate: DOMAIN_PROFILE.into(),
                    object: self.spec_digest.clone(),
                    source: format!(
                        "domain-witness:{}",
                        digest(&bytes(&f.witness).expect("bounded witness"))
                    ),
                })
                .collect(),
            diagnostics: gaps.into_iter().collect(),
        };
        let report = guardengine::integration::evaluate_bounded(&self.contract, &facts)
            .map_err(|e| e.to_string())?;
        let mut obligations = Vec::new();
        for (kind, key, aggregate) in self
            .invariants
            .iter()
            .map(|i| ("invariant", i.id.as_str(), i.aggregate.as_str()))
            .chain(
                self.transitions
                    .iter()
                    .map(|t| ("transition", t.id.as_str(), t.aggregate.as_str())),
            )
        {
            obligations.push(TestObligation {
                id: format!(
                    "ag-test-obligation:{}",
                    digest(&bytes(&(&self.spec_digest, kind, key))?)
                ),
                domain_profile: DOMAIN_PROFILE.into(),
                baseline_digest: baseline.content_digest().into(),
                source_digest: analysis.source_digest().into(),
                declaration: key.into(),
                aggregate: aggregate.into(),
                kind: kind.into(),
                execution: TestExecution::NotRun,
            });
        }
        Ok(DomainEvaluation {
            api_version: "archguard.domain-evaluation/v1alpha1",
            domain_specification_digest: self.spec_digest.clone(),
            provider_profile: self.provider.clone(),
            source_digest: analysis.source_digest().into(),
            capture_digest: digest(analysis.capture()),
            baseline_digest: baseline.content_digest().into(),
            contract: self.contract.clone(),
            facts,
            report,
            findings,
            obligations,
        })
    }
}
#[derive(Serialize)]
pub struct DomainFinding {
    pub invariant: String,
    pub witness: ConfirmedEdge,
}
#[derive(Serialize, Debug, PartialEq, Eq)]
pub enum TestExecution {
    NotRun,
}
#[derive(Serialize)]
pub struct TestObligation {
    pub id: String,
    pub domain_profile: String,
    pub baseline_digest: String,
    pub source_digest: String,
    pub declaration: String,
    pub aggregate: String,
    pub kind: String,
    pub execution: TestExecution,
}
/// Local domain evidence, not an envelope or TestGuard execution report.
#[derive(Serialize)]
pub struct DomainEvaluation {
    api_version: &'static str,
    domain_specification_digest: String,
    provider_profile: String,
    source_digest: String,
    capture_digest: String,
    baseline_digest: String,
    contract: GuardContract,
    facts: GuardFacts,
    report: GuardReport,
    findings: Vec<DomainFinding>,
    obligations: Vec<TestObligation>,
}
impl DomainEvaluation {
    pub fn report(&self) -> &GuardReport {
        &self.report
    }
    pub fn facts(&self) -> &GuardFacts {
        &self.facts
    }
    pub fn contract(&self) -> &GuardContract {
        &self.contract
    }
    pub fn findings(&self) -> &[DomainFinding] {
        &self.findings
    }
    pub fn test_obligations(&self) -> &[TestObligation] {
        &self.obligations
    }
}
