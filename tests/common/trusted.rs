use crate::common::{Temp, evidence};
use archguard::integration::{projection::*, trust::*};
use guardengine::{
    Enforcement,
    integration::{GuardRunEnvelope, eligibility::*},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::AtomicBool,
};
pub const NOW: i64 = 2_000_000_000;
#[derive(Clone)]
pub struct Authority {
    pub producer_principal: String,
    pub producer_validity: Validity,
    pub producer_digest: Option<String>,
    pub unavailable: bool,
    pub approvals: BTreeMap<String, ApprovalRecord>,
}
impl Default for Authority {
    fn default() -> Self {
        Self {
            producer_principal: "fixture-producer".into(),
            producer_validity: Validity {
                issued_at: 0,
                expires_at: NOW + 100,
                revoked: false,
            },
            producer_digest: None,
            unavailable: false,
            approvals: BTreeMap::new(),
        }
    }
}
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        e: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if self.unavailable {
            return Err(AuthorityError::Unavailable);
        }
        Ok(ProducerRecord {
            principal: self.producer_principal.clone(),
            producer: e.producer.clone(),
            envelope_digest: self.producer_digest.clone().unwrap_or_else(|| d.into()),
            validity: self.producer_validity.clone(),
        })
    }
    fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
        if self.unavailable {
            return Err(AuthorityError::Unavailable);
        }
        self.approvals
            .get(r)
            .cloned()
            .ok_or(AuthorityError::Untrusted)
    }
}
pub fn fixture(
    partial: bool,
    review: bool,
) -> (Temp, EvidenceBundle, ConsumptionPolicy, Authority) {
    let root = Temp::new();
    root.copy_fixture(if review { "forbidden" } else { "allowed" });
    let mut c = evidence::contract();
    if !partial && !review {
        let guardengine::GuardAssertion::ForbidRelation { object, .. } =
            &mut c.spec.rules[0].assertion;
        *object = "agent-job".into();
    }
    if review {
        c.spec.rules[0].enforcement = Enforcement::Review;
    }
    let b = CargoEvidence::prepare(
        &root.0,
        ProtectedCargoPolicy::freeze(c, vec![]).unwrap(),
        evidence::context(&root.0),
    )
    .unwrap()
    .run(&AtomicBool::new(false))
    .unwrap();
    let p = ConsumptionPolicy {
        eligibility: EligibilityPolicy {
            binding: b.envelope.binding.clone(),
            producer: b.envelope.producer.clone(),
            required_scopes: b.envelope.coverage.required_scopes.clone(),
            contract_digest: b
                .envelope
                .artifacts
                .contract
                .as_ref()
                .unwrap()
                .digest
                .clone(),
            action: "merge".into(),
            producer_principals: BTreeSet::from(["fixture-producer".into()]),
            approval_principals: BTreeMap::new(),
        },
        required_capabilities: BTreeSet::from([
            "cargo.declarations".into(),
            "cargo.depends_on".into(),
        ]),
        required_approvals: BTreeMap::new(),
    };
    (root, b, p, Authority::default())
}
pub fn approve(p: &mut ConsumptionPolicy, a: &mut Authority, purpose: &str) {
    let reference = format!("fixture-authority:{purpose}");
    p.eligibility
        .approval_principals
        .insert(purpose.into(), BTreeSet::from(["fixture-approver".into()]));
    p.required_approvals
        .insert(purpose.into(), reference.clone());
    a.approvals.insert(
        reference,
        ApprovalRecord {
            principal: "fixture-approver".into(),
            purpose: purpose.into(),
            action: p.eligibility.action.clone(),
            binding: p.eligibility.binding.clone(),
            contract_digest: p.eligibility.contract_digest.clone(),
            validity: Validity {
                issued_at: 0,
                expires_at: NOW + 100,
                revoked: false,
            },
        },
    );
}
