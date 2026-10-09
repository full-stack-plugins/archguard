//! Native, source-bound local index. No external index parser or authority import.
use super::java::JavaAnalysis;
use crate::{
    domain::model::*,
    integration::projection::{bytes, digest},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
pub const INDEX_VERSION: &str = "archguard.native-source-index/v1";
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IndexIdentity {
    pub index_version: String,
    pub model_version: String,
    pub provider_profile: String,
    pub source_digest: String,
    pub capture_digest: String,
}
#[derive(Serialize)]
pub(crate) struct IndexData {
    pub identity: IndexIdentity,
    pub symbols: Vec<Symbol>,
    pub edges: Vec<ConfirmedEdge>,
    pub unknowns: Vec<UnknownRelation>,
    pub classfiles: BTreeMap<String, String>,
    pub external: BTreeSet<String>,
    pub static_complete: bool,
    pub gaps: Vec<(String, String)>,
}
/// Only real native analysis creates this index; serialized bytes are archival, not credentials.
/// ```compile_fail
/// let index: archguard::analysis::codegraph::SourceIndex = serde_json::from_str("{}").unwrap();
/// ```
pub struct SourceIndex {
    pub(crate) data: IndexData,
    raw: Vec<u8>,
    digest: String,
}
pub(crate) fn admit(value: &impl Serialize) -> Result<(), String> {
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(b.len())
                .filter(|n| *n <= 1024 * 1024)
                .ok_or_else(|| std::io::Error::other("index budget"))?;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Count(0), value).map_err(|_| "source index/diff byte budget".into())
}
impl SourceIndex {
    pub fn from_java(analysis: &JavaAnalysis) -> Result<Self, String> {
        let observation = analysis.observation();
        observation.bounded_input_bytes().map_err(str::to_string)?;
        let symbols = observation.symbols().collect::<Vec<_>>();
        admit(&(
            &symbols,
            observation.edges(),
            observation.unknowns(),
            analysis.classfile_digests(),
            analysis.external_references(),
            analysis.gaps(),
        ))?;
        if analysis.capture().len() > 1024 * 1024 {
            return Err("native capture budget".into());
        }
        let data = IndexData {
            identity: IndexIdentity {
                index_version: INDEX_VERSION.into(),
                model_version: observation.model_version().into(),
                provider_profile: observation.provider_profile().into(),
                source_digest: analysis.source_digest().into(),
                capture_digest: digest(analysis.capture()),
            },
            symbols: symbols.into_iter().cloned().collect(),
            edges: observation.edges().to_vec(),
            unknowns: observation.unknowns().to_vec(),
            classfiles: analysis.classfile_digests().clone(),
            external: analysis.external_references().clone(),
            static_complete: observation.is_complete(Relation::DependsOn),
            gaps: analysis
                .gaps()
                .iter()
                .map(|g| (g.class_name.clone(), g.reason.clone()))
                .collect(),
        };
        admit(&data)?;
        let raw = bytes(&data)?;
        let digest = digest(&raw);
        Ok(Self { data, raw, digest })
    }
    pub fn identity(&self) -> &IndexIdentity {
        &self.data.identity
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn artifact_uri(&self) -> String {
        format!(
            "artifact://archguard/source-index/{}",
            self.digest.trim_start_matches("sha256:")
        )
    }
    pub fn bytes(&self) -> &[u8] {
        &self.raw
    }
    pub(crate) fn verify(&self, expected: &IndexIdentity) -> Result<(), String> {
        admit(expected)?;
        if expected != self.identity()
            || expected.index_version != INDEX_VERSION
            || expected.model_version != MODEL_VERSION
        {
            return Err("stale or foreign source index; reindex required".into());
        }
        Ok(())
    }
}
