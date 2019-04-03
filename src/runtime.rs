use core::mem;
use mwgc::Heap;

use crate::constant_pool::ConstantPool;
use crate::error::{ErrorCode, RuntimeError, ToError};
use crate::stack_frame::{StackFrame, StackFrameMutRef};


#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Opcode {
    Break = 0x00,
    Nop = 0x01,
    LoadSlot = 0x08,                    // load slot #B from obj A -> A
    Unknown = 0xff,
}

impl Opcode {
    // why isn't this automatic or derivable?
    pub fn from_u8(n: u8) -> Opcode {
        unsafe { mem::transmute(n) }
    }
}


/// format of a code block:
///   - u8: # of locals <= 63
///   - u8: # of stack slots <= 63
///   - bytecode
struct Code<'rom> {
    pub local_count: u8,
    pub max_stack: u8,
    pub bytecode: &'rom [u8],
}

impl<'rom> Code<'rom> {
    pub fn from_data(data: &'rom [u8]) -> Option<Code<'rom>> {
        if data.len() < 3 || data[0] > 63 || data[1] > 63 { return None }
        let local_count = data[0];
        let max_stack = data[1];
        Some(Code { local_count, max_stack, bytecode: &data[2..] })
    }
}


pub struct Runtime<'runtime, 'rom, 'heap> {
    pool: ConstantPool<'rom>,
    heap: &'runtime mut Heap<'heap>,
}

impl<'runtime, 'rom, 'heap> Runtime<'runtime, 'rom, 'heap> {
    pub fn new(pool: ConstantPool<'rom>, heap: &'runtime mut Heap<'heap>) -> Runtime<'runtime, 'rom, 'heap> {
        Runtime { pool, heap }
    }

    pub fn execute(&mut self, code_index: usize) -> Result<usize, RuntimeError<'rom, 'heap>> {
        let mut frame = self.frame_from_code(code_index, None, &[])?;

        macro_rules! fail {
            ($code: expr) => {
                return Err(frame.to_error($code));
            };
        }

        while (frame.pc as usize) < frame.bytecode.len() {
            match Opcode::from_u8(frame.bytecode[frame.pc as usize]) {
                Opcode::Break => {
                    fail!(ErrorCode::Break);
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
            frame.pc += 1;
        }

        // got here? nothing to return.
        Ok(0)
    }

    pub fn frame_from_code(
        &mut self,
        code_index: usize,
        mut prev_frame: Option<StackFrameMutRef<'rom, 'heap>>,
        args: &[usize]
    ) -> Result<StackFrameMutRef<'rom, 'heap>, RuntimeError<'rom, 'heap>> {
        let code = self.pool.get(code_index).and_then(|data| Code::from_data(data)).ok_or(
            prev_frame.to_error(ErrorCode::InvalidCodeObject)
        )?;

        let mut frame = StackFrame::allocate(self.heap, code.local_count, code.max_stack, code.bytecode).ok_or(
            prev_frame.to_error(ErrorCode::OutOfMemory)
        )?;

        frame.previous = prev_frame;
        frame.start_locals(args).map(|_| frame)
    }
}
