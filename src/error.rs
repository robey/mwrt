use core::fmt;
use crate::stack_frame::{RuntimeContext, StackFrame};

#[derive(Debug, PartialEq)]
pub enum ErrorCode {
    // these errors indicate that there's something wrong with your bytecode generator:
    InvalidCodeObject = 1,
    Unaligned,
    InvalidAddress,
    InvalidSize,
    OutOfBounds,
    UnknownOpcode,
    TruncatedCode,
    StackUnderflow,
    StackOverflow,
    LocalsOverflow,

    // these errors are resource constraints:
    OutOfMemory,
    TimeExceeded,
    CyclesExceeded,

    // these errors were invoked by your code object intentionally:
    Break,
}

pub struct RuntimeError {
    pub code: ErrorCode,
    pub frame: *const StackFrame,
}

impl RuntimeError {
    pub fn new(code: ErrorCode) -> RuntimeError {
        RuntimeError { code, frame: core::ptr::null() }
    }

    pub fn from<'a, 'rom, 'heap>(code: ErrorCode, context: &'a RuntimeContext<'rom, 'heap>) -> RuntimeError {
        RuntimeError { code, frame: context.frame as *const StackFrame }
    }
}

// this is only safe if the heap is still around:
impl<'heap> fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.code)?;
        if let Some(frame) = unsafe { self.frame.as_ref() } {
            if f.alternate() {
                write!(f, " at {:#?}", frame)?;
            } else {
                write!(f, " at {:?}", frame)?;
            }
        }
        Ok(())
    }
}
