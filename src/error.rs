use crate::stack_frame::StackFrameRef;

pub enum ErrorCode {
    // these errors indicate that there's something wrong with your bytecode generator:
    InvalidCodeObject = 1,
    UnknownOpcode,
    StackUnderflow,
    StackOverflow,
    LocalsOverflow,

    // these errors are resource constraints:
    OutOfMemory,

    // these errors were invoked by your code object intentionally:
    Break,
}

pub struct RuntimeError<'rom, 'heap> {
    pub code: ErrorCode,
    pub frame: Option<StackFrameRef<'rom, 'heap>>,
}

impl<'rom, 'heap> RuntimeError<'rom, 'heap> {
    pub fn new(code: ErrorCode, frame: Option<StackFrameRef<'rom, 'heap>>) -> RuntimeError<'rom, 'heap> {
        RuntimeError { code, frame }
    }
}

pub trait ToError<'rom, 'heap> {
    fn to_error(&mut self, code: ErrorCode) -> RuntimeError<'rom, 'heap>;
}
