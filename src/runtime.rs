use core::mem;
use mwgc::Heap;
use crate::stack_frame::StackFrame;


pub enum ErrorCode {
    // these errors indicate that there's something wrong with your bytecode generator:
    UnknownOpcode = 1,
    StackUnderflow,
    StackOverflow,
}

pub struct RuntimeError<'rom, 'heap> {
    pub code: ErrorCode,
    pub bytecode: &'rom [u8],
    pub bytecode_index: usize,
    pub frame: &'heap mut StackFrame<'heap>,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Opcode {
    LoadSlot = 0x10,
}

impl Opcode {
    // why isn't this automatic or derivable?
    pub fn from_u8(n: u8) -> Opcode {
        unsafe { mem::transmute(n) }
    }
}


pub struct Runtime<'heap> {
    heap: &'heap mut Heap<'heap>,
}

impl<'heap> Runtime<'heap> {
    pub fn execute<'rom>(
        bytecode: &'rom [u8], frame: &'heap mut StackFrame<'heap>
    ) -> Result<usize, RuntimeError<'rom, 'heap>> {
        let mut i = 0;
        while i < bytecode.len() {
            match Opcode::from_u8(bytecode[i]) {
                Opcode::LoadSlot => {

                },
                _ => {
                    return Err(RuntimeError { code: ErrorCode::UnknownOpcode, bytecode, bytecode_index: i, frame });
                }
            };
            i += 1;
        }

        // got here? nothing to return.
        Ok(0)
    }

    // fn error(code: ErrorCode, )
}
