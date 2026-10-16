use crate::{
    architecture_model::ArchitectureModel, baseline_record::BaselineRecord,
    model_change::ModelChange,
};
use serde_json::Value;
use std::collections::BTreeSet;

/// 需求当前分析输入，集中保存基线、候选与原生引用，供检查/复验共用。
pub(crate) struct ChangeInspection {
    pub base_digest: String,
    pub baseline: BaselineRecord,
    pub change: ModelChange,
    pub candidate: ArchitectureModel,
    pub seeds: BTreeSet<String>,
    pub specification: Value,
}
