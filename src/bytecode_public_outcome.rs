use crate::bytecode_public_result::BytecodePublicResult;
use guard_protocol::RunSequence;
/// 公共组合入口的真实执行或单飞命中；Existing不会生成新的质量结果。无Java对应类型。
#[derive(Debug)]
pub enum BytecodePublicOutcome {
    Existing { sequence: RunSequence },
    Finished(Box<BytecodePublicResult>),
}
