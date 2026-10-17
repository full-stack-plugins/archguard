use crate::bytecode_receipt::BytecodeReceipt;
use guard_protocol::RunSequence;
/// 幂等命中不会再次执行；新报告仍只是专业技术结论。无Java对应类型。
#[derive(Debug)]
pub enum BytecodeRunOutcome {
    Existing { sequence: RunSequence },
    Finished(Box<BytecodeReceipt>),
}
