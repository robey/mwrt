use core::mem;
use mwgc::Heap;
use crate::constant_pool::ConstantPool;
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
    Break = 0x00,
    Nop = 0x01,
    LoadSlot = 0x08,                    // load slot #B from obj A -> A
}

impl Opcode {
    // why isn't this automatic or derivable?
    pub fn from_u8(n: u8) -> Opcode {
        unsafe { mem::transmute(n) }
    }
}




pub struct Runtime<'rom, 'heap> {
    constants: ConstantPool<'rom>,
    heap: &'heap mut Heap<'heap>,
}

impl<'rom, 'heap> Runtime<'rom, 'heap> {
    pub fn execute(
        bytecode: &'rom [u8], frame: &'heap mut StackFrame<'heap>
    ) -> Result<usize, RuntimeError<'rom, 'heap>> {
        let mut i = 0;

        macro_rules! fail {
            ($code: expr) => {
                return Err(RuntimeError { code: $code, bytecode, bytecode_index: i, frame });
            };
        }

        while i < bytecode.len() {
            match Opcode::from_u8(bytecode[i]) {
                Opcode::Break => {
                    // FIXME
                },
                Opcode::Nop => {
                    // nothing
                },
                Opcode::LoadSlot => {

                },
                _ => {
                    fail!(ErrorCode::UnknownOpcode);
                }
            };
            i += 1;
        }

        // got here? nothing to return.
        Ok(0)
    }

    // fn error(code: ErrorCode, )
}
