use serde::Deserialize;
use std::collections::BTreeMap;

/// 显式静态 Maven profile 场景，不求值环境激活；无 Java 对应类。
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MavenScenario {
    pub(crate) profiles: BTreeMap<String, bool>,
}
