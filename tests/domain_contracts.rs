mod common;
use archguard::{
    analysis::java::{JavaProfile, JavaToolchain},
    domain::{baseline::ArchitectureBaseline, contracts::*, model::*},
};
use guardengine::{
    Decision, Enforcement,
    integration::{GuardRunEnvelope, Producer, RunBinding, eligibility::*},
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
    sync::OnceLock,
};
const JDK: &str = "/workspace/guard-toolchain/jdk21/usr/lib/jvm/java-21-openjdk-amd64";
fn tools() -> &'static JavaToolchain {
    static TOOLS: OnceLock<JavaToolchain> = OnceLock::new();
    TOOLS.get_or_init(|| JavaToolchain::freeze(Path::new(JDK)).unwrap())
}
fn profile() -> JavaProfile {
    JavaProfile::freeze(
        BTreeMap::from([
            ("app/AppService".into(), "app".into()),
            ("domain/Aggregate".into(), "domain".into()),
            ("domain/State".into(), "domain".into()),
        ]),
        BTreeSet::new(),
    )
    .unwrap()
}
fn symbol(name: &str) -> SymbolId {
    let module = if name.starts_with("app/") {
        "app"
    } else {
        "domain"
    };
    SymbolId::new(Language::Java, module, "", SymbolKind::Type, name, "").unwrap()
}
fn model(enforcement: Enforcement, kind: InvariantKind) -> DomainContract {
    build(enforcement, kind).unwrap()
}
fn build(enforcement: Enforcement, kind: InvariantKind) -> Result<DomainContract, String> {
    let method = SymbolId::new(
        Language::Java,
        "domain",
        "domain/Aggregate",
        SymbolKind::Method,
        "cancel",
        "()V",
    )
    .unwrap();
    DomainContract::freeze(
        &tools().provider_id(&profile()),
        vec![Context {
            id: "task-context".into(),
            members: BTreeSet::from([
                symbol("app/AppService"),
                symbol("domain/Aggregate"),
                symbol("domain/State"),
                method.clone(),
            ]),
        }],
        vec![Aggregate {
            id: "task".into(),
            context: "task-context".into(),
            entry_type: symbol("domain/Aggregate"),
            internal_state_type: symbol("domain/State"),
        }],
        vec![Invariant {
            id: "state-access".into(),
            aggregate: "task".into(),
            enforcement,
            kind,
        }],
        vec![Transition {
            id: "cancel".into(),
            aggregate: "task".into(),
            method,
            from_state: "active".into(),
            to_state: "cancelled".into(),
        }],
    )
}
fn compile(bypass: bool) -> common::Temp {
    let root = common::Temp::new();
    std::fs::create_dir(root.0.join("app")).unwrap();
    std::fs::create_dir(root.0.join("domain")).unwrap();
    std::fs::write(
        root.0.join("domain/State.java"),
        "package domain; public class State { public static void reset() {} }",
    )
    .unwrap();
    std::fs::write(
        root.0.join("domain/Aggregate.java"),
        "package domain; public class Aggregate { public static void cancel() { State.reset(); } }",
    )
    .unwrap();
    std::fs::write(root.0.join("app/AppService.java"),if bypass{"package app; public class AppService { public static void cancelTask() { domain.State.reset(); } }"}else{"package app; public class AppService { public static void cancelTask() { domain.Aggregate.cancel(); } }"}).unwrap();
    let out = Command::new(Path::new(JDK).join("bin/javac"))
        .args([
            "-J-Xmx128m",
            "-J-XX:ActiveProcessorCount=2",
            "-proc:none",
            "--release",
            "21",
            "-g",
            "-d",
        ])
        .arg(&root.0)
        .args([
            root.0.join("domain/State.java"),
            root.0.join("domain/Aggregate.java"),
            root.0.join("app/AppService.java"),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    root
}
struct Authority {
    record: ApprovalRecord,
    revoked: Cell<bool>,
    unavailable: Cell<bool>,
}
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        _: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        Err(AuthorityError::Untrusted)
    }
    fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
        if self.unavailable.get() || r != "external:domain-approval" {
            return Err(AuthorityError::Unavailable);
        }
        let mut record = self.record.clone();
        record.validity.revoked = self.revoked.get();
        Ok(record)
    }
}
fn approve(model: &DomainContract) -> (ArchitectureBaseline, EligibilityPolicy, Authority) {
    approve_obligations(model, model.required_obligations().clone())
}
fn approve_obligations(
    model: &DomainContract,
    obligations: BTreeSet<String>,
) -> (ArchitectureBaseline, EligibilityPolicy, Authority) {
    let binding = RunBinding {
        repo_id: "controller-repo".into(),
        task_id: "domain-task".into(),
        worktree_id: "baseline-source".into(),
        requirement_ids: vec!["R1".into()],
        candidate_oid: "a".repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: format!("sha256:{}", "c".repeat(64)),
        baseline_digest: None,
    };
    let review = ArchitectureBaseline::draft(
        binding.clone(),
        "protected:domain-source",
        BTreeMap::from([(
            "ADR1".into(),
            b"explicit static type access contract".to_vec(),
        )]),
        model.contract(),
        obligations,
    )
    .unwrap()
    .submit()
    .unwrap();
    let mut bound = binding;
    bound.baseline_digest = Some(review.content_digest().into());
    let policy = EligibilityPolicy {
        binding: bound.clone(),
        producer: Producer {
            guard: "archguard".into(),
            version: "1".into(),
            analyzer_id: "domain".into(),
            analyzer_version: "1".into(),
        },
        required_scopes: vec!["R1".into()],
        contract_digest: review.contract_digest().into(),
        action: "architecture-baseline:approve".into(),
        producer_principals: BTreeSet::from(["fixture".into()]),
        approval_principals: BTreeMap::from([(
            "architecture-baseline".into(),
            BTreeSet::from(["external-reviewer".into()]),
        )]),
    };
    let authority = Authority {
        record: ApprovalRecord {
            principal: "external-reviewer".into(),
            purpose: "architecture-baseline".into(),
            action: policy.action.clone(),
            binding: bound,
            contract_digest: policy.contract_digest.clone(),
            validity: Validity {
                issued_at: 10,
                expires_at: 100,
                revoked: false,
            },
        },
        revoked: Cell::new(false),
        unavailable: Cell::new(false),
    };
    let approved = review
        .approve("external:domain-approval", &policy, &authority, 50)
        .unwrap();
    (approved, policy, authority)
}
fn capture(
    label: &str,
    analysis: &archguard::analysis::java::JavaAnalysis,
    result: &DomainEvaluation,
) {
    if let Some(directory) = std::env::var_os("ARCHGUARD_DOMAIN_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(format!("{label}.json")),
            serde_json::to_vec_pretty(result).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join(format!("{label}.capture.tsv")),
            analysis.capture(),
        )
        .unwrap();
    }
}
fn verified(e: &DomainEvaluation) {
    assert_eq!(
        guardengine::evaluate(e.contract(), e.facts()).unwrap(),
        *e.report()
    );
    assert!(
        e.test_obligations()
            .iter()
            .all(|o| o.execution == TestExecution::NotRun)
    );
}
#[test]
fn actual_java_static_coordination_and_direct_state_access_follow_approved_contract() {
    let contract = model(Enforcement::Enforce, InvariantKind::StaticStateAccess);
    let (baseline, policy, authority) = approve(&contract);
    let legal = compile(false);
    let legal = tools().analyze(&legal.0, &profile()).unwrap();
    let result = contract
        .evaluate_java(&legal, &baseline, &policy, &authority, 50)
        .unwrap();
    verified(&result);
    assert_eq!(result.report().decision, Decision::Allow);
    assert!(result.findings().is_empty());
    capture("legal-static", &legal, &result);
    let bad = compile(true);
    let analysis = tools().analyze(&bad.0, &profile()).unwrap();
    let result = contract
        .evaluate_java(&analysis, &baseline, &policy, &authority, 50)
        .unwrap();
    verified(&result);
    assert_eq!(result.report().decision, Decision::Block);
    assert_eq!(result.findings().len(), 1);
    assert_eq!(result.findings()[0].witness.from, symbol("app/AppService"));
    assert!(
        result.findings()[0]
            .witness
            .source
            .path()
            .contains("captures")
    );
    assert_eq!(result.test_obligations().len(), 2);
    capture("forbidden-static", &analysis, &result);
    let (weakened, weak_policy, weak_authority) = approve_obligations(
        &contract,
        BTreeSet::from([contract.required_obligations().first().unwrap().clone()]),
    );
    assert!(
        contract
            .evaluate_java(&analysis, &weakened, &weak_policy, &weak_authority, 50)
            .is_err()
    );
    authority.revoked.set(true);
    assert!(
        contract
            .evaluate_java(&analysis, &baseline, &policy, &authority, 50)
            .is_err()
    );
    authority.revoked.set(false);
    authority.unavailable.set(true);
    assert!(
        contract
            .evaluate_java(&analysis, &baseline, &policy, &authority, 50)
            .is_err()
    );
    authority.unavailable.set(false);
    assert!(
        contract
            .evaluate_java(&analysis, &baseline, &policy, &authority, 100)
            .is_err()
    );
    std::fs::remove_file(bad.0.join("domain/State.class")).unwrap();
    let partial = tools().analyze(&bad.0, &profile()).unwrap();
    let result = contract
        .evaluate_java(&partial, &baseline, &policy, &authority, 50)
        .unwrap();
    assert_eq!(
        result.facts().completeness,
        guardengine::Completeness::Partial
    );
    assert_eq!(result.report().decision, Decision::Block);
}
#[test]
fn transitions_remain_unknown_and_heuristic_rules_do_not_become_enforce() {
    assert!(build(Enforcement::Enforce, InvariantKind::HeuristicStateAccess).is_err());
    let input = compile(true);
    let analysis = tools().analyze(&input.0, &profile()).unwrap();
    assert!(!analysis.observation().is_complete(Relation::Calls));
    let transition = model(
        Enforcement::Enforce,
        InvariantKind::TransitionEntry {
            transition: "cancel".into(),
        },
    );
    let (b, p, a) = approve(&transition);
    let result = transition.evaluate_java(&analysis, &b, &p, &a, 50).unwrap();
    assert_eq!(
        result.facts().completeness,
        guardengine::Completeness::Partial
    );
    assert_eq!(result.report().decision, Decision::Block);
    assert!(result.findings().is_empty());
    capture("transition-gap", &analysis, &result);
    let advisory = model(Enforcement::Review, InvariantKind::HeuristicStateAccess);
    let (b, p, a) = approve(&advisory);
    let result = advisory.evaluate_java(&analysis, &b, &p, &a, 50).unwrap();
    assert_eq!(result.report().decision, Decision::RequireApproval);
    verified(&result);
    let changed = model(Enforcement::Advise, InvariantKind::HeuristicStateAccess);
    assert!(changed.evaluate_java(&analysis, &b, &p, &a, 50).is_err());
    let (b, p, a) = approve(&changed);
    let result = changed.evaluate_java(&analysis, &b, &p, &a, 50).unwrap();
    assert_eq!(result.report().decision, Decision::Allow);
    assert_eq!(result.findings().len(), 1);
}
#[test]
fn model_validation_rejects_oversized_metadata_and_preserves_semantic_order() {
    let members = BTreeSet::from([symbol("domain/Aggregate"), symbol("domain/State")]);
    let aggregate = Aggregate {
        id: "task".into(),
        context: "context".into(),
        entry_type: symbol("domain/Aggregate"),
        internal_state_type: symbol("domain/State"),
    };
    let invariant = Invariant {
        id: "one".into(),
        aggregate: "task".into(),
        enforcement: Enforcement::Enforce,
        kind: InvariantKind::StaticStateAccess,
    };
    assert!(
        DomainContract::freeze(
            "native",
            vec![Context {
                id: "x".repeat(17 * 1024 * 1024),
                members: members.clone()
            }],
            vec![aggregate.clone()],
            vec![invariant.clone()],
            vec![]
        )
        .is_err()
    );
    let mut second = invariant.clone();
    second.id = "two".into();
    let context = Context {
        id: "context".into(),
        members,
    };
    let a = DomainContract::freeze(
        "native",
        vec![context.clone()],
        vec![aggregate.clone()],
        vec![invariant.clone(), second.clone()],
        vec![],
    )
    .unwrap();
    let b = DomainContract::freeze(
        "native",
        vec![context],
        vec![aggregate],
        vec![second, invariant],
        vec![],
    )
    .unwrap();
    assert_eq!(a.digest(), b.digest());
    assert_eq!(a.required_obligations(), b.required_obligations());
}
