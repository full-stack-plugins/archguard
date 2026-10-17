use serde::{Deserialize, Serialize};
/// 执行前声明的Maven模块和完整编译类型清单。对应Java ModuleInput。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BytecodeModuleInput {
    pub path: String,
    pub expected_types: Vec<String>,
}
