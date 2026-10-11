mod common;
use archguard::{
    analysis::java::{JavaProfile, JavaToolchain},
    domain::{baseline::ArchitectureBaseline, contracts::*, model::*},
};
use guardengine::{
    Enforcement,
    integration::{GuardRunEnvelope, Producer, RunBinding, eligibility::*},
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    process::Command,
    sync::OnceLock,
};
fn tools() -> &'static JavaToolchain {
    static TOOLS: OnceLock<JavaToolchain> = OnceLock::new();
    TOOLS.get_or_init(|| JavaToolchain::freeze(&common::jdk_home()).unwrap())
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
    let out = Command::new(common::jdk_home().join("bin/javac"))
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

fn approve_obligations(
    model: &DomainContract,
    obligations: BTreeSet<String>,
    index: &SourceIndex,
) -> (ArchitectureBaseline, EligibilityPolicy, Authority) {
    let binding = RunBinding {
        repo_id: "controller-repo".into(),
        task_id: "domain-task".into(),
        worktree_id: "baseline-source".into(),
        requirement_ids: vec!["R1".into()],
        candidate_oid: "a".repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: index.identity().source_digest.clone(),
        baseline_digest: None,
    };
    let review = ArchitectureBaseline::draft(
        binding.clone(),
        &index.artifact_uri(),
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

use archguard::{analysis::codegraph::SourceIndex, domain::diff::*};
#[test]
fn actual_rename_and_shared_consumers_never_prove_zero_impact() {
    let old = compile(false);
    let new = compile(false);
    let path = new.0.join("domain/Aggregate.java");
    std::fs::write(&path, "package domain; public class Aggregate { public static void renamed() { State.reset(); } }").unwrap();
    std::fs::write(new.0.join("app/AppService.java"), "package app; public class AppService { public static void cancelTask() { domain.Aggregate.renamed(); } }").unwrap();
    let output = Command::new(common::jdk_home().join("bin/javac"))
        .args(["-proc:none", "--release", "21", "-g", "-d"])
        .arg(&new.0)
        .args([
            new.0.join("domain/State.java"),
            path,
            new.0.join("app/AppService.java"),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let before = tools().analyze(&old.0, &profile()).unwrap();
    let after = tools().analyze(&new.0, &profile()).unwrap();
    let left = SourceIndex::from_java(&before).unwrap();
    let right = SourceIndex::from_java(&after).unwrap();
    let model = build(Enforcement::Enforce, InvariantKind::StaticStateAccess).unwrap();
    let (baseline, policy, authority) =
        approve_obligations(&model, model.required_obligations().clone(), &left);
    let refs = vec![
        RequirementImpact {
            requirement: "R1".into(),
            symbol: symbol("domain/Aggregate"),
            obligations: model.required_obligations().clone(),
        },
        RequirementImpact {
            requirement: "R2".into(),
            symbol: symbol("domain/Aggregate"),
            obligations: model.required_obligations().clone(),
        },
    ];
    let out = compare(
        &left,
        &right,
        left.identity(),
        right.identity(),
        &baseline,
        &policy,
        &authority,
        50,
        &refs,
    )
    .unwrap();
    assert!(out.changes.iter().any(|c| c.kind == ChangeKind::Removed));
    assert!(out.changes.iter().any(|c| c.kind == ChangeKind::Added));
    assert!(!out.unknowns.is_empty());
    assert!(!out.changed_classfiles.is_empty());
    assert!(!out.shared_dependencies.is_empty());
    assert!(!out.static_consumers.is_empty());
    let mut stale = right.identity().clone();
    stale.source_digest = left.identity().source_digest.clone();
    assert!(
        compare(
            &left,
            &right,
            left.identity(),
            &stale,
            &baseline,
            &policy,
            &authority,
            50,
            &refs
        )
        .is_err()
    );
    let mut old_version = right.identity().clone();
    old_version.model_version = "old-index/v0".into();
    assert!(
        compare(
            &left,
            &right,
            left.identity(),
            &old_version,
            &baseline,
            &policy,
            &authority,
            50,
            &refs
        )
        .is_err()
    );
    let mut foreign_profile = right.identity().clone();
    foreign_profile.provider_profile = "foreign-provider".into();
    assert!(
        compare(
            &left,
            &right,
            left.identity(),
            &foreign_profile,
            &baseline,
            &policy,
            &authority,
            50,
            &refs
        )
        .is_err()
    );
    let excessive = vec![RequirementImpact {
        requirement: "x".repeat(17 * 1024 * 1024),
        symbol: symbol("domain/Aggregate"),
        obligations: model.required_obligations().clone(),
    }];
    assert!(
        compare(
            &left,
            &right,
            left.identity(),
            right.identity(),
            &baseline,
            &policy,
            &authority,
            50,
            &excessive
        )
        .is_err()
    );
    if let Some(dir) = std::env::var_os("ARCHGUARD_DIFF_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("diff.json"),
            serde_json::to_vec_pretty(&out).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("baseline.capture.tsv"), before.capture()).unwrap();
        std::fs::write(dir.join("candidate.capture.tsv"), after.capture()).unwrap();
        std::fs::write(dir.join("baseline.index.json"), left.bytes()).unwrap();
        std::fs::write(dir.join("candidate.index.json"), right.bytes()).unwrap();
    }
    let absent = vec![
        RequirementImpact {
            requirement: "R1".into(),
            symbol: symbol("missing/Api"),
            obligations: model.required_obligations().clone(),
        },
        RequirementImpact {
            requirement: "R2".into(),
            symbol: symbol("missing/Api"),
            obligations: model.required_obligations().clone(),
        },
    ];
    let missing = compare(
        &left,
        &right,
        left.identity(),
        right.identity(),
        &baseline,
        &policy,
        &authority,
        50,
        &absent,
    )
    .unwrap();
    assert!(missing.shared_dependencies.is_empty());
    assert!(
        missing
            .unknowns
            .iter()
            .any(|g| g.contains("absent from both"))
    );
    authority.revoked.set(true);
    assert!(
        compare(
            &left,
            &right,
            left.identity(),
            right.identity(),
            &baseline,
            &policy,
            &authority,
            50,
            &refs
        )
        .is_err()
    );
}

#[test]
fn actual_module_reassignment_is_not_guessed_as_same_ownership() {
    let root = compile(false);
    let before = tools().analyze(&root.0, &profile()).unwrap();
    let changed = JavaProfile::freeze(
        BTreeMap::from([
            ("app/AppService".into(), "app".into()),
            ("domain/Aggregate".into(), "new-owner".into()),
            ("domain/State".into(), "domain".into()),
        ]),
        BTreeSet::new(),
    )
    .unwrap();
    let after = tools().analyze(&root.0, &changed).unwrap();
    let left = SourceIndex::from_java(&before).unwrap();
    let right = SourceIndex::from_java(&after).unwrap();
    let model = build(Enforcement::Enforce, InvariantKind::StaticStateAccess).unwrap();
    let (baseline, policy, authority) =
        approve_obligations(&model, model.required_obligations().clone(), &left);
    let out = compare(
        &left,
        &right,
        left.identity(),
        right.identity(),
        &baseline,
        &policy,
        &authority,
        50,
        &[],
    )
    .unwrap();
    assert!(
        out.changes
            .iter()
            .any(|c| c.kind == ChangeKind::Removed && c.before.unwrap().id.module() == "domain")
    );
    assert!(
        out.changes
            .iter()
            .any(|c| c.kind == ChangeKind::Added && c.after.unwrap().id.module() == "new-owner")
    );
    assert!(
        out.unknowns
            .iter()
            .any(|g| g.contains("ownership attribution unknown"))
    );
    assert!(
        compare(
            &right,
            &left,
            right.identity(),
            left.identity(),
            &baseline,
            &policy,
            &authority,
            50,
            &[]
        )
        .is_err()
    );
    let unchanged = compare(
        &left,
        &left,
        left.identity(),
        left.identity(),
        &baseline,
        &policy,
        &authority,
        50,
        &[],
    )
    .unwrap();
    assert!(unchanged.changes.is_empty());
    assert!(!unchanged.unknowns.is_empty());
    assert!(
        compare(
            &left,
            &right,
            left.identity(),
            right.identity(),
            &baseline,
            &policy,
            &authority,
            101,
            &[]
        )
        .is_err()
    );
}

fn compile_dispatch(changed: bool) -> common::Temp {
    let root = common::Temp::new();
    std::fs::create_dir(root.0.join("app")).unwrap();
    std::fs::create_dir(root.0.join("domain")).unwrap();
    std::fs::write(
        root.0.join("domain/State.java"),
        "package domain; public interface State { int value(); }",
    )
    .unwrap();
    std::fs::write(
        root.0.join("domain/Aggregate.java"),
        format!("package domain; public class Aggregate implements State {{ public int value() {{ return {}; }} }}", if changed { 2 } else { 1 }),
    ).unwrap();
    std::fs::write(
        root.0.join("app/AppService.java"),
        "package app; public class AppService { public int viaInterface(domain.State state) { return state.value(); } public int viaVirtual(domain.Aggregate aggregate) { return aggregate.value(); } }",
    ).unwrap();
    let out = Command::new(common::jdk_home().join("bin/javac"))
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

#[test]
fn actual_interface_and_virtual_dispatch_diff_retains_native_unknown_provenance() {
    use sha2::{Digest, Sha256};
    let old = compile_dispatch(false);
    let new = compile_dispatch(true);
    let before = tools().analyze(&old.0, &profile()).unwrap();
    let after = tools().analyze(&new.0, &profile()).unwrap();
    let left = SourceIndex::from_java(&before).unwrap();
    let right = SourceIndex::from_java(&after).unwrap();
    let model = build(Enforcement::Enforce, InvariantKind::StaticStateAccess).unwrap();
    let (baseline, policy, authority) =
        approve_obligations(&model, model.required_obligations().clone(), &left);
    let out = compare(
        &left,
        &right,
        left.identity(),
        right.identity(),
        &baseline,
        &policy,
        &authority,
        50,
        &[],
    )
    .unwrap();
    assert_ne!(
        left.identity().source_digest,
        right.identity().source_digest
    );
    assert_eq!(out.baseline_index, left.digest());
    assert_eq!(out.candidate_index, right.digest());
    assert!(out.changed_classfiles.contains(&"domain/Aggregate"));
    assert!(
        out.unknowns
            .contains(&"dynamic dispatch and complete Calls consumers unavailable")
    );
    assert!(
        out.unknowns
            .contains(&"native unresolved relations retained in referenced index")
    );
    for (analysis, index) in [(&before, &left), (&after, &right)] {
        assert_eq!(index.identity().source_digest, analysis.source_digest());
        assert_eq!(
            index.identity().provider_profile,
            tools().provider_id(&profile())
        );
        assert_eq!(
            index.identity().capture_digest,
            format!("sha256:{:x}", Sha256::digest(analysis.capture()))
        );
        let archived: serde_json::Value = serde_json::from_slice(index.bytes()).unwrap();
        assert_eq!(
            archived["unknowns"],
            serde_json::to_value(analysis.observation().unknowns()).unwrap()
        );
        for (name, signature, reason) in [
            (
                "viaInterface",
                "(Ldomain/State;)I",
                "invokeinterface dispatch unresolved",
            ),
            (
                "viaVirtual",
                "(Ldomain/Aggregate;)I",
                "invokevirtual dispatch unresolved",
            ),
        ] {
            let caller = SymbolId::new(
                Language::Java,
                "app",
                "app/AppService",
                SymbolKind::Method,
                name,
                signature,
            )
            .unwrap();
            let unknown = analysis
                .observation()
                .unknowns()
                .iter()
                .find(|u| u.from == caller && u.reason == reason)
                .expect("actual native dispatch unknown");
            assert_eq!(unknown.relation, Relation::Calls);
            assert_eq!(unknown.source.path(), "captures/java-bytecode.tsv");
            let line = std::str::from_utf8(analysis.capture())
                .unwrap()
                .lines()
                .nth(unknown.source.start().0 as usize - 1)
                .unwrap();
            assert!(line.starts_with("U\t"));
            assert!(
                line.contains(
                    &reason
                        .as_bytes()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                )
            );
            assert!(
                !analysis
                    .observation()
                    .edges()
                    .iter()
                    .any(|e| e.relation == Relation::Calls && e.from == caller)
            );
        }
    }
    if let Some(dir) = std::env::var_os("ARCHGUARD_DIFF_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir).join("dispatch");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, data) in [
            ("baseline.capture.tsv", before.capture()),
            ("candidate.capture.tsv", after.capture()),
            ("baseline.index.json", left.bytes()),
            ("candidate.index.json", right.bytes()),
        ] {
            std::fs::write(dir.join(name), data).unwrap();
        }
        std::fs::write(
            dir.join("diff.json"),
            serde_json::to_vec_pretty(&out).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn stale_index_refusal_identifies_frozen_reindex_scope_without_raw_input() {
    use sha2::{Digest, Sha256};
    let root = compile(false);
    let analysis = tools().analyze(&root.0, &profile()).unwrap();
    let index = SourceIndex::from_java(&analysis).unwrap();
    let model = build(Enforcement::Enforce, InvariantKind::StaticStateAccess).unwrap();
    let (baseline, policy, authority) =
        approve_obligations(&model, model.required_obligations().clone(), &index);
    let marker = "arbitrary-private-source\nnot-diagnostic-content";
    let mut expected = index.identity().clone();
    expected.source_digest = marker.into();
    expected.provider_profile = marker.into();
    let error = compare(
        &index,
        &index,
        index.identity(),
        &expected,
        &baseline,
        &policy,
        &authority,
        50,
        &[],
    )
    .err()
    .expect("stale input must refuse comparison");
    assert!(
        error.contains("reindex required; scope=entire frozen Java profile"),
        "{error}"
    );
    assert!(
        error.contains("indexed modules=[\"app\",\"domain\"]"),
        "{error}"
    );
    assert!(
        error.contains(&format!(
            "expected identity=sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&expected).unwrap())
        )),
        "{error}"
    );
    assert!(
        error.contains(&format!(
            "expected provider/profile=sha256:{:x}",
            Sha256::digest(expected.provider_profile.as_bytes())
        )),
        "{error}"
    );
    assert!(!error.contains(marker));
    assert!(!error.contains("arbitrary-private-source"));
}
