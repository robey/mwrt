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


// what to do after executing a bytecode
enum Disposition<'rom, 'heap> {
    Continue(u16),  // keep going, possibly across a jump
    End,            // ran out of bytes
    Error(RuntimeError<'rom, 'heap>),
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

    pub fn execute(&mut self, code_index: usize, args: &[usize]) -> Result<usize, RuntimeError<'rom, 'heap>> {
        let frame = self.frame_from_code(code_index, None, args)?;
        loop {
            let d = self.execute_one(frame);
            match d {
                Disposition::Continue(next_pc) => {
                    frame.pc = next_pc;
                },
                Disposition::End => {
                    // got here? nothing to return.
                    return Ok(0);
                },
                Disposition::Error(err) => {
                    return Err(err);
                }
            }
        }
    }

    fn execute_one(&mut self, frame: &StackFrame<'rom, 'heap>) -> Disposition<'rom, 'heap> {
        let mut next_pc = frame.pc as usize;
        if next_pc >= frame.bytecode.len() { return Disposition::End }
        let instruction = frame.bytecode[next_pc];
        next_pc += 1;

        // FIXME: read args

        match Opcode::from_u8(instruction) {
            Opcode::Break => {
                return Disposition::Error(frame.to_error(ErrorCode::Break));
            },
            Opcode::Nop => {
                // nothing
            },

            Opcode::LoadSlot => {

            },

            _ => {
                return Disposition::Error(frame.to_error(ErrorCode::UnknownOpcode));
            }
        }

        Disposition::Continue(next_pc as u16)
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
            self.heap, code_index, code.local_count, code.max_stack, code.bytecode
        ).ok_or(
            prev_frame.to_error(ErrorCode::OutOfMemory)
        )?;

        frame.previous = prev_frame;
        frame.start_locals(args).map(|_| frame)
    }
}


#[cfg(test)]
mod tests {
    use mwgc::Heap;
    use crate::constant_pool::ConstantPool;
    use super::{Opcode, Runtime};

    #[test]
    fn unknown() {
        let mut data: [u8; 256] = [0; 256];
        let mut heap = Heap::from_bytes(&mut data);
        // constant pool: 1 code block of "unknown" (ff)
        let pool = ConstantPool::new(&[ 3, 1, 1, 0xff ]);
        let mut runtime = Runtime::new(pool, &mut heap);
        assert_eq!(format!("{:?}", runtime.execute(0, &[])), "Err(UnknownOpcode at [frame code=0 pc=0 sp=0])");
    }

    #[test]
    fn break_instruction() {
        let mut data: [u8; 256] = [0; 256];
        let mut heap = Heap::from_bytes(&mut data);
        let pool = ConstantPool::new(&[ 3, 1, 1, Opcode::Break as u8 ]);
        let mut runtime = Runtime::new(pool, &mut heap);

        assert_eq!(format!("{:?}", runtime.execute(0, &[])), "Err(Break at [frame code=0 pc=0 sp=0])");
    }

    #[test]
    fn skip_nop() {
        let mut data: [u8; 256] = [0; 256];
        let mut heap = Heap::from_bytes(&mut data);
        let pool = ConstantPool::new(&[ 4, 1, 1, Opcode::Nop as u8, Opcode::Break as u8 ]);
        let mut runtime = Runtime::new(pool, &mut heap);

        assert_eq!(format!("{:?}", runtime.execute(0, &[])), "Err(Break at [frame code=0 pc=1 sp=0])");
    }
}
