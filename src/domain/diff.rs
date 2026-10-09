//! Explanatory native diff; static evidence never becomes an eligibility grant.
use super::{baseline::ArchitectureBaseline, model::*};
use crate::{
    analysis::codegraph::{IndexIdentity, SourceIndex, admit},
    integration::projection::{bytes, digest},
};
use guardengine::integration::eligibility::{AuthorityProvider, EligibilityPolicy};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Serialize)]
pub struct RequirementImpact {
    pub requirement: String,
    pub symbol: SymbolId,
    pub obligations: BTreeSet<String>,
}
#[derive(Serialize, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Removed,
    CaptureLocationChanged,
}
#[derive(Serialize)]
pub struct SymbolChange<'a> {
    pub kind: ChangeKind,
    pub before: Option<&'a Symbol>,
    pub after: Option<&'a Symbol>,
}
#[derive(Serialize)]
pub struct StaticConsumer<'a> {
    pub side: &'static str,
    pub edge: &'a ConfirmedEdge,
}
#[derive(Serialize)]
pub struct SharedDependency<'a> {
    pub symbol: &'a SymbolId,
    pub requirements: Vec<&'a RequirementImpact>,
}
#[derive(Serialize)]
pub struct DesignDiff<'a> {
    pub version: &'static str,
    pub baseline_record: &'a str,
    pub baseline_index: &'a str,
    pub candidate_index: &'a str,
    pub policy_digest: String,
    pub changes: Vec<SymbolChange<'a>>,
    pub changed_classfiles: Vec<&'a str>,
    pub static_consumers: Vec<StaticConsumer<'a>>,
    pub shared_dependencies: Vec<SharedDependency<'a>>,
    pub requirement_impacts: &'a [RequirementImpact],
    pub unknowns: Vec<&'static str>,
}
/// `expected_*`, mapping and clock are protected controller inputs. Native bytes do not authenticate them.
#[allow(clippy::too_many_arguments)]
pub fn compare<'a>(
    before: &'a SourceIndex,
    after: &'a SourceIndex,
    expected_before: &IndexIdentity,
    expected_after: &IndexIdentity,
    baseline: &'a ArchitectureBaseline,
    policy: &EligibilityPolicy,
    authority: &dyn AuthorityProvider,
    now: i64,
    requirements: &'a [RequirementImpact],
) -> Result<DesignDiff<'a>, String> {
    admit(&(expected_before, expected_after, requirements))?;
    if requirements.len() > 64 {
        return Err("diff requirement count budget".into());
    }
    before.verify(expected_before)?;
    after.verify(expected_after)?;
    baseline.qualify(policy, authority, now)?;
    if baseline.binding().source_snapshot_digest != before.identity().source_digest
        || baseline.source_ref() != before.artifact_uri()
    {
        return Err("approved baseline does not bind actual native index".into());
    }
    let mut seen = BTreeSet::new();
    for r in requirements {
        if r.requirement.trim().is_empty()
            || r.requirement.contains('\0')
            || r.requirement.len() > 256
            || r.obligations.is_empty()
            || !r.obligations.is_subset(baseline.obligations())
            || !seen.insert((&r.requirement, &r.symbol))
        {
            return Err("invalid diff requirement/obligation mapping".into());
        }
    }
    let old = before
        .data
        .symbols
        .iter()
        .map(|s| (&s.id, s))
        .collect::<BTreeMap<_, _>>();
    let new = after
        .data
        .symbols
        .iter()
        .map(|s| (&s.id, s))
        .collect::<BTreeMap<_, _>>();
    let changed_classfiles = before
        .data
        .classfiles
        .keys()
        .chain(after.data.classfiles.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|name| before.data.classfiles.get(*name) != after.data.classfiles.get(*name))
        .map(String::as_str)
        .collect();
    let mut changes = Vec::new();
    for (id, symbol) in &old {
        match new.get(id) {
            None => changes.push(SymbolChange {
                kind: ChangeKind::Removed,
                before: Some(symbol),
                after: None,
            }),
            Some(next) if symbol.source != next.source => changes.push(SymbolChange {
                kind: ChangeKind::CaptureLocationChanged,
                before: Some(symbol),
                after: Some(next),
            }),
            _ => (),
        }
    }
    for (id, symbol) in &new {
        if !old.contains_key(id) {
            changes.push(SymbolChange {
                kind: ChangeKind::Added,
                before: None,
                after: Some(symbol),
            })
        }
    }
    let mut unknowns = vec![
        "public visibility unavailable in native profile",
        "method body semantic equivalence unavailable",
        "dynamic dispatch and complete Calls consumers unavailable",
        "consumer universe outside frozen native index unknown",
        "rename/owner equivalence unavailable; identity changes remain removal/addition",
    ];
    if before.identity().provider_profile != after.identity().provider_profile {
        unknowns.push("different provider/configuration profiles; ownership attribution unknown")
    }
    if !before.data.static_complete || !after.data.static_complete {
        unknowns.push("incomplete static dependency scope")
    }
    if !before.data.unknowns.is_empty() || !after.data.unknowns.is_empty() {
        unknowns.push("native unresolved relations retained in referenced index")
    }
    if !before.data.gaps.is_empty() || !after.data.gaps.is_empty() {
        unknowns.push("native parse/coverage gaps retained in referenced index")
    }
    if !before.data.external.is_empty() || !after.data.external.is_empty() {
        unknowns.push("external native references lack indexed consumer coverage")
    }
    let mut grouped: BTreeMap<&SymbolId, Vec<&RequirementImpact>> = BTreeMap::new();
    for r in requirements {
        if !old.contains_key(&r.symbol) && !new.contains_key(&r.symbol) {
            unknowns.push("requirement symbol absent from both indexes");
            continue;
        }
        grouped.entry(&r.symbol).or_default().push(r);
    }
    let shared_dependencies = grouped
        .into_iter()
        .filter(|(_, r)| r.len() > 1)
        .map(|(symbol, requirements)| SharedDependency {
            symbol,
            requirements,
        })
        .collect();
    // Whole frozen static consumer inventories are retained; method call impact is not inferred.
    let static_consumers = before
        .data
        .edges
        .iter()
        .map(|edge| StaticConsumer {
            side: "baseline",
            edge,
        })
        .chain(after.data.edges.iter().map(|edge| StaticConsumer {
            side: "candidate",
            edge,
        }))
        .filter(|e| e.edge.relation == Relation::DependsOn)
        .collect();
    let policy_digest = digest(&bytes(&(
        "archguard.design-diff-policy/v1",
        expected_before,
        expected_after,
        requirements,
    ))?);
    let result = DesignDiff {
        version: "archguard.native-design-diff/v1",
        baseline_record: baseline.digest(),
        baseline_index: before.digest(),
        candidate_index: after.digest(),
        policy_digest,
        changes,
        changed_classfiles,
        static_consumers,
        shared_dependencies,
        requirement_impacts: requirements,
        unknowns,
    };
    // All output references borrow already-admitted storage; reject expansion before serialization ownership.
    admit(&result)?;
    Ok(result)
}
