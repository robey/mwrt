use core::mem;
use mwgc::Heap;

use crate::constant_pool::ConstantPool;
use crate::decode_int::decode_sint;
use crate::error::{ErrorCode, RuntimeError, ToError};
use crate::stack_frame::{StackFrame, StackFrameMutRef};


#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Opcode {
    Break = 0x00,
    Nop = 0x01,
    Return = 0x02,
    LoadSlot = 0x08,                    // load slot #B from obj A -> A
    Immediate = 0x10,                   // N1 -> S1
    Unknown = 0xff,
}

// opcodes 0x1X have one immediate; 0x2X have two
const FIRST_N1_OPCODE: u8 = 0x10;
const FIRST_N2_OPCODE: u8 = 0x20;
const LAST_N_OPCODE: u8 = 0x30;

impl Opcode {
    // why isn't this automatic or derivable?
    pub fn from_u8(n: u8) -> Opcode {
        unsafe { mem::transmute(n) }
    }
}


// what to do after executing a bytecode
enum Disposition {
    Continue(u16),  // keep going, possibly across a jump
    End,            // ran out of bytes
    Return(usize),
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


pub struct Runtime<'rom, 'heap> {
    pool: ConstantPool<'rom>,
    heap: Heap<'heap>,
}

impl<'rom, 'heap> Runtime<'rom, 'heap> {
    pub fn new(pool: ConstantPool<'rom>, heap_data: &'heap mut [u8]) -> Runtime<'rom, 'heap> {
        Runtime { pool, heap: Heap::from_bytes(heap_data) }
    }

    pub fn execute(
        &mut self, code_index: usize, args: &[usize], results: &mut [usize]
    ) -> Result<usize, RuntimeError<'rom, 'heap>> {
        let frame = self.frame_from_code(code_index, None, args)?;
        loop {
            let d = self.execute_one(frame)?;
            match d {
                Disposition::Continue(next_pc) => {
                    frame.pc = next_pc;
                },
                Disposition::End => {
                    // ran out of bytecodes? nothing to return.
                    return Ok(0);
                },
                Disposition::Return(count) => {
                    let stack_results = frame.stack_from(count, &self.heap)?;
                    let n: usize = if count < results.len() { count } else { results.len() };
                    for i in 0 .. n { results[i] = stack_results[i] }
                    return Ok(count);
                }
            }
        }
    }

    fn execute_one(&mut self, frame: &mut StackFrame<'rom, 'heap>) -> Result<Disposition, RuntimeError<'rom, 'heap>> {
        let mut next_pc = frame.pc as usize;
        if next_pc >= frame.bytecode.len() { return Ok(Disposition::End) }
        let instruction = frame.bytecode[next_pc];
        next_pc += 1;

        // immediates?
        let mut n1: isize = 0;
        let mut n2: isize = 0;
        if instruction >= FIRST_N1_OPCODE && instruction < LAST_N_OPCODE {
            let d1 = decode_sint(frame.bytecode, next_pc).ok_or_else(|| frame.to_error(ErrorCode::TruncatedCode))?;
            n1 = d1.value;
            next_pc = d1.new_index;
            if instruction >= FIRST_N2_OPCODE {
                let d2 = decode_sint(frame.bytecode, next_pc).ok_or_else(|| frame.to_error(ErrorCode::TruncatedCode))?;
                n2 = d2.value;
                next_pc = d2.new_index;
            }
        }

        match Opcode::from_u8(instruction) {
            // zero immediates:

            Opcode::Break => {
                return Err(frame.to_error(ErrorCode::Break));
            },
            Opcode::Nop => {
                // nothing
            },
            Opcode::Return => {
                let count = frame.get(&mut self.heap)?;
                return Ok(Disposition::Return(count));
            },

            // one immediate:

            Opcode::Immediate => {
                frame.put(n1 as usize, &mut self.heap)?;
            },

            Opcode::LoadSlot => {

            },

            _ => {
                return Err(frame.to_error(ErrorCode::UnknownOpcode));
            }
        }

        Ok(Disposition::Continue(next_pc as u16))
    }

    // look up a code object in the constant pool, allocate a new stack frame
    // for it, and load the arguments into locals.
    pub fn frame_from_code(
        &mut self,
        code_index: usize,
        prev_frame: Option<StackFrameMutRef<'rom, 'heap>>,
        args: &[usize]
    ) -> Result<StackFrameMutRef<'rom, 'heap>, RuntimeError<'rom, 'heap>> {
        let code = self.pool.get(code_index).and_then(|data| Code::from_data(data)).ok_or(
            prev_frame.to_error(ErrorCode::InvalidCodeObject)
        )?;

        let mut frame = StackFrame::allocate(
            &mut self.heap, code_index, code.local_count, code.max_stack, code.bytecode
        ).ok_or(
            prev_frame.to_error(ErrorCode::OutOfMemory)
        )?;

        frame.previous = prev_frame;
        frame.start_locals(args).map(|_| frame)
    }
}
