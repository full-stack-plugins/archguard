//! Append-only attempt/CAS port. The in-memory implementation is not durable authority.
use serde::Serialize;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Target {
    pub repo_id: String,
    pub task_id: String,
    pub requirement_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptRecord {
    pub run_id: String,
    pub target: Target,
    pub generation: u64,
    pub content_digest: String,
    pub envelope_digest: String,
    pub eligible: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreError {
    Conflict,
    Stale,
    Invalid,
    UnknownAttempt,
    GenerationExhausted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendOutcome {
    Inserted,
    IdenticalReplay,
}
pub trait AttemptStore {
    fn advance(
        &mut self,
        target: Target,
        expected_generation: u64,
        content_digest: String,
        run_id: String,
    ) -> Result<u64, StoreError>;
    fn append(&mut self, record: AttemptRecord) -> Result<AppendOutcome, StoreError>;
    fn publish(&mut self, run_id: &str) -> Result<(), StoreError>;
    fn current(&self, target: &Target) -> Option<&AttemptRecord>;
    fn history(&self, target: &Target) -> Vec<&AttemptRecord>;
}
#[derive(Clone)]
struct Registration {
    target: Target,
    generation: u64,
    content_digest: String,
}
#[derive(Default)]
pub struct InMemoryAttemptStore {
    registrations: std::collections::BTreeMap<String, Registration>,
    generations: std::collections::BTreeMap<Target, u64>,
    records: std::collections::BTreeMap<String, AttemptRecord>,
    order: Vec<String>,
    published: std::collections::BTreeMap<Target, String>,
}
impl AttemptStore for InMemoryAttemptStore {
    fn advance(
        &mut self,
        target: Target,
        expected_generation: u64,
        content_digest: String,
        run_id: String,
    ) -> Result<u64, StoreError> {
        if target.repo_id.trim().is_empty()
            || target.task_id.trim().is_empty()
            || target.requirement_ids.is_empty()
            || target.requirement_ids.iter().any(|r| r.trim().is_empty())
            || !target.requirement_ids.windows(2).all(|w| w[0] < w[1])
            || run_id.trim().is_empty()
            || !valid_digest(&content_digest)
        {
            return Err(StoreError::Invalid);
        }
        if self.registrations.contains_key(&run_id) {
            return Err(StoreError::Conflict);
        }
        let current = self.generations.get(&target).copied().unwrap_or(0);
        if expected_generation != current {
            return Err(StoreError::Stale);
        }
        let generation = current
            .checked_add(1)
            .ok_or(StoreError::GenerationExhausted)?;
        self.registrations.insert(
            run_id,
            Registration {
                target: target.clone(),
                generation,
                content_digest,
            },
        );
        self.generations.insert(target.clone(), generation);
        self.published.remove(&target);
        Ok(generation)
    }
    fn append(&mut self, record: AttemptRecord) -> Result<AppendOutcome, StoreError> {
        if let Some(old) = self.records.get(&record.run_id) {
            return if old == &record {
                Ok(AppendOutcome::IdenticalReplay)
            } else {
                Err(StoreError::Conflict)
            };
        }
        let registration = self
            .registrations
            .get(&record.run_id)
            .ok_or(StoreError::UnknownAttempt)?;
        if registration.target != record.target
            || registration.generation != record.generation
            || registration.content_digest != record.content_digest
        {
            return Err(StoreError::Conflict);
        }
        if !valid_digest(&record.envelope_digest) {
            return Err(StoreError::Invalid);
        }
        self.order.push(record.run_id.clone());
        self.records.insert(record.run_id.clone(), record);
        Ok(AppendOutcome::Inserted)
    }
    fn publish(&mut self, run_id: &str) -> Result<(), StoreError> {
        let record = self.records.get(run_id).ok_or(StoreError::UnknownAttempt)?;
        if self.generations.get(&record.target) != Some(&record.generation) {
            return Err(StoreError::Stale);
        }
        self.published
            .insert(record.target.clone(), run_id.to_owned());
        Ok(())
    }
    fn current(&self, target: &Target) -> Option<&AttemptRecord> {
        self.published
            .get(target)
            .and_then(|id| self.records.get(id))
    }
    fn history(&self, target: &Target) -> Vec<&AttemptRecord> {
        self.order
            .iter()
            .filter_map(|id| self.records.get(id))
            .filter(|r| &r.target == target)
            .collect()
    }
}
fn valid_digest(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(|d| {
        d.len() == 64
            && d.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
