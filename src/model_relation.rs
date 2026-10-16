use serde::{Deserialize, Serialize};

/// 模型声明关系，使用稳定元素 ID 连接，不代表运行时调用已被观测。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelRelation {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
}
