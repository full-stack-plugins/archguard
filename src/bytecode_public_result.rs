use crate::{bytecode_artifact_ref::BytecodeArtifactRef, bytecode_receipt::BytecodeReceipt};
use guard_protocol::GuardResult;
/// 专业公共协议结果与其私有原件；仍未授信，不含GateDecision或Grant。无Java对应类型。
#[derive(Debug)]
pub struct BytecodePublicResult {
    pub receipt: BytecodeReceipt,
    pub result: GuardResult,
    pub artifact: BytecodeArtifactRef,
}
