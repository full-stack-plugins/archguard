use crate::maven_workspace::MavenWorkspace;
use guardengine::{
    API_VERSION_V2, AnalysisCoverage, AnalyzerIdentity, Completeness, GuardAnalyzer, GuardError,
    GuardFacts, GuardSubject,
};
use std::path::Path;

/// 提取 Maven reactor 的声明模块依赖；不执行构建，不声称覆盖 Java 调用语义。无 Java 对应类。
#[derive(Default)]
pub struct MavenWorkspaceAnalyzer;
impl GuardAnalyzer for MavenWorkspaceAnalyzer {
    /// 分析给定项目及主体标识，返回带覆盖声明和源快照的事实；模型无法解析时返回 partial。
    fn analyze(&self, project_root: &Path, subject_id: &str) -> Result<GuardFacts, GuardError> {
        let root = project_root
            .canonicalize()
            .map_err(|e| GuardError::Input(e.to_string()))?;
        let mut workspace = MavenWorkspace::new();
        let (facts, diagnostics, completeness) = match workspace.collect(&root) {
            Ok(facts) => (facts, vec![], Completeness::Complete),
            Err(error) => (vec![], vec![error], Completeness::Partial),
        };
        workspace.entities.sort();
        workspace.entities.dedup();
        Ok(GuardFacts {api_version:API_VERSION_V2.into(),kind:"GuardFacts".into(),analyzer:AnalyzerIdentity {id:"archguard.maven.reactor".into(),version:env!("CARGO_PKG_VERSION").into()},subject:GuardSubject {id:subject_id.into(),snapshot_digest:workspace.inputs.digest(&root)},completeness,coverage:Some(AnalysisCoverage {entities:workspace.entities,predicates:vec!["depends_on".into()],scope:"Maven reactor declared dependencies, all declared scopes; local parent inheritance only; no Java imports, transitive external resolution or runtime calls".into()}),facts,diagnostics})
    }
}
