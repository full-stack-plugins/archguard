//! Local, provider-declared module graph rules. This module neither extracts
//! language semantics nor authenticates protected policy/provider declarations.
use super::model::*;
use guardengine::{
    API_VERSION, AnalyzerIdentity, Completeness, ContractMetadata, ContractSpec, Enforcement,
    GuardAssertion, GuardContract, GuardFact, GuardFacts, GuardReport, GuardRule, GuardSubject,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const SYSTEM_PROFILE: &str = "archguard.system-rules/v1alpha1";
const MAX_BYTES: usize = 4 * 1024 * 1024;
fn bounded_digest<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = crate::integration::projection::bytes(value)?;
    Ok(crate::integration::projection::digest(&bytes))
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ModuleKey {
    language: Language,
    module: String,
}
impl ModuleKey {
    pub fn new(language: Language, module: &str) -> Result<Self, String> {
        if module.trim().is_empty() || module.len() > 256 {
            return Err("invalid module identity".into());
        }
        Ok(Self {
            language,
            module: module.into(),
        })
    }
    pub fn language(&self) -> Language {
        self.language
    }
    pub fn module(&self) -> &str {
        &self.module
    }
    fn of(symbol: &SymbolId) -> Result<Self, String> {
        Self::new(symbol.language(), symbol.module())
    }
}
#[derive(Debug, Clone, Serialize)]
pub enum SystemRuleKind {
    /// Higher levels may depend on lower/equal levels. Every protected module
    /// must have one level. Same-module implementation edges are internal.
    Layers {
        #[serde(serialize_with = "levels_json")]
        levels: BTreeMap<ModuleKey, u32>,
    },
    ForbiddenDirection {
        from: ModuleKey,
        to: ModuleKey,
    },
    /// Module-level cycles; one deterministic real witness is sufficient.
    Acyclic,
}
fn levels_json<S: serde::Serializer>(
    levels: &BTreeMap<ModuleKey, u32>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    levels.iter().collect::<Vec<_>>().serialize(serializer)
}
#[derive(Debug, Clone, Serialize)]
pub struct SystemRule {
    pub id: String,
    pub enforcement: Enforcement,
    pub kind: SystemRuleKind,
}
/// Owned immutable policy. Approval of these values remains a controller duty.
pub struct ProtectedSystemPolicy {
    providers: BTreeMap<Language, String>,
    modules: BTreeSet<ModuleKey>,
    rules: Vec<SystemRule>,
    digest: String,
    contract: GuardContract,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SystemFinding {
    pub rule_id: String,
    pub path: Vec<ConfirmedEdge>,
}
/// Read-only local result. No producer identity, envelope or authorization claim.
pub struct SystemEvaluation {
    findings: Vec<SystemFinding>,
    contract: GuardContract,
    facts: GuardFacts,
    report: GuardReport,
}
impl SystemEvaluation {
    pub fn findings(&self) -> &[SystemFinding] {
        &self.findings
    }
    pub fn contract(&self) -> &GuardContract {
        &self.contract
    }
    pub fn facts(&self) -> &GuardFacts {
        &self.facts
    }
    pub fn report(&self) -> &GuardReport {
        &self.report
    }
}
impl ProtectedSystemPolicy {
    pub fn freeze(
        providers: BTreeMap<Language, String>,
        modules: BTreeSet<ModuleKey>,
        relation: Relation,
        mut rules: Vec<SystemRule>,
    ) -> Result<Self, String> {
        if relation != Relation::DependsOn {
            return Err("unsupported system rule relation".into());
        }
        if modules.is_empty()
            || modules.len() > 1024
            || rules.is_empty()
            || rules.len() > 64
            || providers.is_empty()
            || providers.len() > 3
            || providers
                .values()
                .any(|p| p.trim().is_empty() || p.len() > 256)
        {
            return Err("invalid or oversized system policy".into());
        }
        if providers.keys().copied().collect::<BTreeSet<_>>()
            != modules.iter().map(|m| m.language).collect()
        {
            return Err("provider scope must equal protected module languages".into());
        }
        let mut policy_bytes = providers.values().map(String::len).sum::<usize>()
            + modules.iter().map(|m| m.module.len()).sum::<usize>();
        let mut ids = BTreeSet::new();
        for rule in &rules {
            policy_bytes += rule.id.len();
            if let SystemRuleKind::Layers { levels } = &rule.kind {
                // Check before serializing or cloning the repeated scope keys.
                if levels.len() > 1024 {
                    return Err("layer policy budget exceeded".into());
                }
                policy_bytes += levels.keys().map(|m| m.module.len() + 32).sum::<usize>();
            }
            if policy_bytes > 1024 * 1024 {
                return Err("policy byte budget exceeded".into());
            }
            if rule.id.trim().is_empty() || rule.id.len() > 256 || !ids.insert(&rule.id) {
                return Err("invalid rule id".into());
            }
            match &rule.kind {
                SystemRuleKind::Layers { levels } => {
                    if levels.len() != modules.len() || levels.keys().ne(modules.iter()) {
                        return Err("layer scope must equal protected modules".into());
                    }
                }
                SystemRuleKind::ForbiddenDirection { from, to } => {
                    if !modules.contains(from) || !modules.contains(to) {
                        return Err("direction endpoint outside protected scope".into());
                    }
                }
                SystemRuleKind::Acyclic => {}
            }
        }
        rules.sort_by(|a, b| a.id.cmp(&b.id));
        let digest = bounded_digest(&(SYSTEM_PROFILE, &providers, &modules, &rules))
            .map_err(|e| e.to_string())?;
        let contract = GuardContract {
            api_version: API_VERSION.into(),
            kind: "GuardContract".into(),
            metadata: ContractMetadata {
                id: SYSTEM_PROFILE.into(),
                revision: digest.clone(),
            },
            spec: ContractSpec {
                rules: rules
                    .iter()
                    .map(|rule| GuardRule {
                        id: rule.id.clone(),
                        description: "ArchGuard evaluated protected module graph rule".into(),
                        enforcement: rule.enforcement.clone(),
                        assertion: GuardAssertion::ForbidRelation {
                            subject: rule.id.clone(),
                            predicate: SYSTEM_PROFILE.into(),
                            object: digest.clone(),
                        },
                    })
                    .collect(),
            },
        };
        contract.validate().map_err(|e| e.to_string())?;
        Ok(Self {
            providers,
            modules,
            rules,
            digest,
            contract,
        })
    }
    pub fn profile_digest(&self) -> &str {
        &self.digest
    }
    pub fn evaluate(
        &self,
        observations: &[LanguageObservation],
    ) -> Result<SystemEvaluation, String> {
        if observations.len() > 3 {
            return Err("observation count budget exceeded".into());
        }
        if observations.iter().map(|o| o.edges().len()).sum::<usize>() > 8192
            || observations
                .iter()
                .map(|o| o.symbols().count())
                .sum::<usize>()
                > 4096
            || observations
                .iter()
                .map(|o| o.unknowns().len())
                .sum::<usize>()
                > 4096
        {
            return Err("combined observation budget exceeded".into());
        }
        let mut input_bytes = 0;
        for o in observations {
            input_bytes += o.bounded_input_bytes()?;
            if input_bytes > MAX_BYTES {
                return Err("observation byte budget exceeded".into());
            }
        }
        let mut by_language = BTreeMap::new();
        for o in observations {
            if self.providers.get(&o.language()).map(String::as_str) != Some(o.provider_profile()) {
                return Err("unsupported provider profile".into());
            }
            if by_language.insert(o.language(), o).is_some() {
                return Err("duplicate language observation".into());
            }
        }
        let mut gaps = BTreeSet::new();
        let mut present = BTreeSet::new();
        let mut edges = BTreeSet::new();
        let mut canonical = Vec::new();
        for (language, profile) in &self.providers {
            let Some(o) = by_language.get(language) else {
                gaps.insert(format!("missing provider: {language:?}/{profile}"));
                continue;
            };
            if !o.is_complete(Relation::DependsOn) {
                gaps.insert(format!("incomplete depends_on: {language:?}/{profile}"));
            }
            for s in o.symbols() {
                let key = ModuleKey::of(&s.id)?;
                if !self.modules.contains(&key) {
                    gaps.insert(format!("unprotected observed module: {key:?}"));
                }
                present.insert(key);
            }
            let mut all_edges = o.edges().iter().collect::<Vec<_>>();
            all_edges.sort();
            all_edges.dedup();
            let mut unknowns = o.unknowns().iter().collect::<Vec<_>>();
            unknowns.sort();
            unknowns.dedup();
            // Canonical source material includes unrelated relations and unresolved
            // provenance, not only the projected violation facts.
            canonical.push((
                language,
                profile,
                o.symbols().collect::<Vec<_>>(),
                all_edges.clone(),
                unknowns,
                o.is_complete(Relation::DependsOn),
                o.is_complete(Relation::Calls),
            ));
            for edge in all_edges {
                if edge.relation != Relation::DependsOn {
                    continue;
                }
                let from = ModuleKey::of(&edge.from)?;
                let to = ModuleKey::of(&edge.to)?;
                // A relation between implementation symbols in one module is not
                // a module dependency; explicit Module->Module self edges count.
                if from == to
                    && !(edge.from.kind() == SymbolKind::Module
                        && edge.to.kind() == SymbolKind::Module)
                {
                    continue;
                }
                edges.insert(edge);
            }
        }
        for m in self.modules.difference(&present) {
            gaps.insert(format!("missing module: {m:?}"));
        }
        // Global bounds also apply across multiple language providers.
        if present.len() > 1024 || edges.len() > 8192 {
            return Err("module graph budget exceeded".into());
        }
        let snapshot_digest =
            bounded_digest(&(SYSTEM_PROFILE, canonical)).map_err(|e| e.to_string())?;
        let edges = edges.into_iter().collect::<Vec<_>>();
        let cycle = first_cycle(&edges)?;
        let mut findings = Vec::new();
        let mut witness_bytes = 0;
        let mut witness_steps = 0;
        let mut append = |rule: &SystemRule, path: &[&ConfirmedEdge]| -> Result<(), String> {
            witness_steps += path.len();
            witness_bytes += path.iter().map(|e| e.input_bytes()).sum::<usize>();
            if witness_steps > 8192 || witness_bytes > MAX_BYTES {
                return Err("finding witness budget exceeded".into());
            }
            findings.push(SystemFinding {
                rule_id: rule.id.clone(),
                path: path.iter().map(|e| (*e).clone()).collect(),
            });
            Ok(())
        };
        for rule in &self.rules {
            match &rule.kind {
                SystemRuleKind::Acyclic => {
                    if !cycle.is_empty() {
                        append(rule, &cycle)?;
                    }
                }
                SystemRuleKind::ForbiddenDirection { from, to } => {
                    for edge in &edges {
                        if ModuleKey::of(&edge.from)? == *from && ModuleKey::of(&edge.to)? == *to {
                            append(rule, &[edge])?;
                        }
                    }
                }
                SystemRuleKind::Layers { levels } => {
                    for edge in &edges {
                        if let (Some(a), Some(b)) = (
                            levels.get(&ModuleKey::of(&edge.from)?),
                            levels.get(&ModuleKey::of(&edge.to)?),
                        ) && a < b
                        {
                            append(rule, &[edge])?;
                        }
                    }
                }
            }
        }
        let mut facts = Vec::new();
        for finding in &findings {
            facts.push(GuardFact {
                subject: finding.rule_id.clone(),
                predicate: SYSTEM_PROFILE.into(),
                object: self.digest.clone(),
                source: format!(
                    "archguard.system.finding:{}",
                    bounded_digest(finding).map_err(|e| e.to_string())?
                ),
            });
        }
        let facts = GuardFacts {
            api_version: API_VERSION.into(),
            kind: "GuardFacts".into(),
            analyzer: AnalyzerIdentity {
                id: SYSTEM_PROFILE.into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            subject: GuardSubject {
                id: "local-language-observations".into(),
                snapshot_digest,
            },
            completeness: if gaps.is_empty() {
                Completeness::Complete
            } else {
                Completeness::Partial
            },
            facts,
            diagnostics: gaps.into_iter().collect(),
        };
        let report = guardengine::integration::evaluate_bounded(&self.contract, &facts)
            .map_err(|e| e.to_string())?;
        Ok(SystemEvaluation {
            findings,
            contract: self.contract.clone(),
            facts,
            report,
        })
    }
}
/// Iterative ordered DFS: first gray-edge gives a genuine contiguous module
/// cycle. Linear traversal, no recursion or all-simple-cycle enumeration.
fn first_cycle<'a>(edges: &[&'a ConfirmedEdge]) -> Result<Vec<&'a ConfirmedEdge>, String> {
    let mut graph: BTreeMap<ModuleKey, Vec<&ConfirmedEdge>> = BTreeMap::new();
    for edge in edges {
        graph
            .entry(ModuleKey::of(&edge.from)?)
            .or_default()
            .push(edge);
        graph.entry(ModuleKey::of(&edge.to)?).or_default();
    }
    let mut colors: BTreeMap<ModuleKey, u8> = BTreeMap::new();
    for root in graph.keys() {
        if colors.contains_key(root) {
            continue;
        }
        let mut stack = vec![(root.clone(), 0usize)];
        let mut path = Vec::new();
        colors.insert(root.clone(), 1);
        while let Some((node, next)) = stack.last_mut() {
            let adjacent = &graph[node];
            if *next == adjacent.len() {
                colors.insert(node.clone(), 2);
                stack.pop();
                if !path.is_empty() {
                    path.pop();
                }
                continue;
            }
            let edge = adjacent[*next];
            *next += 1;
            let target = ModuleKey::of(&edge.to)?;
            match colors.get(&target) {
                Some(1) => {
                    let start = stack
                        .iter()
                        .position(|(n, _)| *n == target)
                        .ok_or("invalid graph traversal")?;
                    let mut cycle = path[start..].to_vec();
                    cycle.push(edge);
                    return Ok(cycle);
                }
                Some(_) => {}
                None => {
                    colors.insert(target.clone(), 1);
                    stack.push((target, 0));
                    path.push(edge);
                }
            }
        }
    }
    Ok(vec![])
}
