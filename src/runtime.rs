use core::mem;
use mwgc::Heap;
use crate::constant_pool::ConstantPool;
use crate::stack_frame::StackFrame;


pub enum ErrorCode {
    // these errors indicate that there's something wrong with your bytecode generator:
    InvalidCodeObject = 1,
    UnknownOpcode,
    StackUnderflow,
    StackOverflow,

    // these errors are resource constraints:
    OutOfMemory,

    // these errors were invoked by your code object intentionally:
    Break,
}

pub struct RuntimeError<'rom, 'heap> {
    pub code: ErrorCode,
    pub frame: Option<&'heap mut StackFrame<'rom, 'heap>>,
}

impl<'rom, 'heap> RuntimeError<'rom, 'heap> {
    pub fn new(code: ErrorCode, frame: Option<&'heap mut StackFrame<'rom, 'heap>>) -> RuntimeError<'rom, 'heap> {
        RuntimeError { code, frame }
    }
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


/// format of a code block:
///   - u8: # of locals <= 63
///   - u8: # of stack slots <= 63
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
    heap: &'heap mut Heap<'heap>,
}

impl<'rom, 'heap> Runtime<'rom, 'heap> {
    pub fn new(pool: ConstantPool<'rom>, heap: &'heap mut Heap<'heap>) -> Runtime<'rom, 'heap> {
        Runtime { pool, heap }
    }

    pub fn execute(&mut self, index: usize) -> Result<usize, RuntimeError<'rom, 'heap>> {
        let code = self.pool.get(index).and_then(|data| Code::from_data(data)).ok_or(
            RuntimeError::new(ErrorCode::InvalidCodeObject, None)
        )?;

        let mut frame = StackFrame::allocate(self.heap, None, code.local_count, code.max_stack, code.bytecode).ok_or(
            RuntimeError::new(ErrorCode::OutOfMemory, None)
        )?;

        macro_rules! fail {
            ($code: expr) => {
                return Err(RuntimeError::new($code, Some(frame)));
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

    // pub frame_from_code()

    // fn error(code: ErrorCode, )
}
