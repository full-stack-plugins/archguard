//! Borrowed-field construction prevents projection fanout before evaluation budgets run.
use super::{IntegrationError, MAX_ARTIFACT_BYTES, adapter::bounded_json_size, fail};

/// Integration-profile builder. Checks each borrowed fact before copying any fields.
/// At most 4096 facts and 16 MiB combined JSON/owned-field estimates are accepted.
/// Any rejected insertion poisons the builder: partial facts cannot be returned as success.
#[derive(Default)]
pub struct FactBudget {
    facts: Vec<crate::GuardFact>,
    charged: usize,
    rejected: bool,
}
impl FactBudget {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push_relation(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        source: &str,
    ) -> Result<(), IntegrationError> {
        self.push_relation_with_source_parts(subject, predicate, object, &[source])
    }
    /// Checks all source fragments before joining them into an owned source string.
    pub fn push_relation_with_source_parts(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        source_parts: &[&str],
    ) -> Result<(), IntegrationError> {
        if self.rejected {
            return fail("fact construction budget already rejected");
        }
        self.rejected = true;
        if source_parts.len() > 4096 {
            return fail("source fragment count exceeded");
        }
        // JSON keys, braces, punctuation and fragment quote overcount are conservative.
        let mut cost = 128usize.saturating_add(2 * std::mem::size_of::<crate::GuardFact>());
        for field in [subject, predicate, object]
            .into_iter()
            .chain(source_parts.iter().copied())
        {
            cost = cost
                .saturating_add(bounded_json_size(&field)?)
                .saturating_add(field.len());
            if self.charged.saturating_add(cost) > MAX_ARTIFACT_BYTES.saturating_sub(2) {
                return fail("fact construction budget exceeded");
            }
        }
        if self.facts.len() >= 4096 {
            return fail("fact construction count exceeded");
        }
        self.facts
            .try_reserve(1)
            .map_err(|_| IntegrationError("fact allocation failed".into()))?;
        self.facts.push(crate::GuardFact {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            source: source_parts.concat(),
        });
        self.charged += cost;
        self.rejected = false;
        Ok(())
    }
    pub fn finish(self) -> Result<Vec<crate::GuardFact>, IntegrationError> {
        if self.rejected {
            return fail("fact construction budget rejected; incomplete projection discarded");
        }
        Ok(self.facts)
    }
}
