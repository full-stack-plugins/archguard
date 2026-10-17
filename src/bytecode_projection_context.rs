use guard_protocol::{Identifier, ProducerIdentity};
/// 控制端显式提供的生产者与运行标识；声明不自动授信。无Java对应类型。
#[derive(Clone, Debug)]
pub struct BytecodeProjectionContext {
    pub producer: ProducerIdentity,
    pub result_id: Identifier,
    pub run_id: Identifier,
    pub operation_id: Identifier,
    pub lineage_id: Identifier,
    pub retention_policy_id: Identifier,
}
