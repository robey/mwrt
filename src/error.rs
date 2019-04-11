use core::fmt;
use crate::stack_frame::StackFrameRef;

#[derive(Debug, PartialEq)]
pub enum ErrorCode {
    // these errors indicate that there's something wrong with your bytecode generator:
    InvalidCodeObject = 1,
    InvalidConstant,
    InvalidAddress,
    InvalidSize,
    UnknownOpcode,
    TruncatedCode,
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

impl<'rom, 'heap> fmt::Debug for RuntimeError<'rom, 'heap> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.code)?;
        if let Some(frame) = &self.frame {
            if f.alternate() {
                write!(f, " at {:#?}", frame)?;
            } else {
                write!(f, " at {:?}", frame)?;
            }
        }
        Ok(())
    }
}

pub trait ToError<'rom, 'heap> {
    fn to_error(&self, code: ErrorCode) -> RuntimeError<'rom, 'heap>;
}
