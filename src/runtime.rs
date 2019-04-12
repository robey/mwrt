use core::{fmt, mem};
use mwgc::Heap;

use crate::constant_pool::ConstantPool;
use crate::decode_int::{decode_sint, decode_unaligned};
use crate::error::{ErrorCode, RuntimeError, ToError};
use crate::opcode::{Binary, FIRST_N1_OPCODE, FIRST_N2_OPCODE, LAST_N_OPCODE, Opcode, Unary};
use crate::stack_frame::{StackFrame, StackFrameMutRef};


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
        &mut self, code_index: u16, args: &[usize], results: &mut [usize]
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
                    let stack_results = frame.get_n(count)?;
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
            Opcode::Dup => {
                let v = frame.get()?;
                frame.put(v)?;
                frame.put(v)?;
            },
            Opcode::Return => {
                let count = frame.get()?;
                return Ok(Disposition::Return(count));
            },
            Opcode::New => {
                let fill = frame.get()?;
                let slots = frame.get()?;
                let obj = self.new_object(slots, fill, frame)?;
                frame.put(obj)?;
            },
            Opcode::Size => {
                let addr = frame.get()?;
                frame.put(self.object_size(addr, frame)?)?;
            },

            // one immediate:

            Opcode::Immediate => {
                frame.put(n1 as usize)?;
            },
            Opcode::Constant => {
                frame.put(((n1 as usize) << 1) | 1)?;
            },
            Opcode::LoadSlotN => {
                let v = self.load_slot(frame.get()?, n1 as usize, frame)?;
                frame.put(v)?;
            },
            Opcode::StoreSlotN => {
                let v = frame.get()?;
                self.store_slot(frame.get()?, n1 as usize, v, frame)?;
            },
            Opcode::LoadLocalN => {
                let locals = frame.locals();
                let n = n1 as usize;
                if n >= locals.len() { return Err(frame.to_error(ErrorCode::OutOfBounds)) }
                frame.put(locals[n])?;
            },
            Opcode::StoreLocalN => {
                let locals = frame.locals_mut();
                let n = n1 as usize;
                if n >= locals.len() { return Err(frame.to_error(ErrorCode::OutOfBounds)) }
                locals[n] = frame.get()?;
            },
            Opcode::Unary => {
                let v = frame.get()?;
                let op = Unary::from_usize(n1 as usize);
                frame.put(self.unary(op, v as isize, frame)? as usize)?;
            },
            Opcode::Binary => {
                let v2 = frame.get()?;
                let v1 = frame.get()?;
                let op = Binary::from_usize(n1 as usize);
                frame.put(self.binary(op, v1 as isize, v2 as isize, frame)? as usize)?;
            },
            Opcode::ReturnN => {
                return Ok(Disposition::Return(n1 as usize));
            },

            Opcode::LoadSlot => {

            },

            // two immediates:

            Opcode::NewNN => {
                let obj = self.new_object(n1 as usize, n2 as usize, frame)?;
                frame.put(obj)?;
            }

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
        code_index: u16,
        prev_frame: Option<StackFrameMutRef<'rom, 'heap>>,
        args: &[usize]
    ) -> Result<StackFrameMutRef<'rom, 'heap>, RuntimeError<'rom, 'heap>> {
        let code = self.pool.get(code_index as usize).and_then(|data| Code::from_data(data)).ok_or(
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

    pub fn object_size(
        &self,
        addr: usize,
        frame: &StackFrame<'rom, 'heap>
    ) -> Result<usize, RuntimeError<'rom, 'heap>> {
        // an object might be serialized into the constant pool, so allow for either form.
        if addr & 1 != 0 {
            // constant pool address
            let obj = self.pool.get(addr >> 1).ok_or_else(|| frame.to_error(ErrorCode::InvalidConstant))?;
            Ok(obj.len() / mem::size_of::<usize>())
        } else {
            // heap address, should be aligned
            Ok(self.heap.size_of(unsafe { &*(addr as *const usize) }) / mem::size_of::<usize>())
        }
    }

    pub fn load_slot(
        &self,
        addr: usize,
        slot: usize,
        frame: &StackFrame<'rom, 'heap>
    ) -> Result<usize, RuntimeError<'rom, 'heap>> {
        // an object might be serialized into the constant pool, so allow for either form.
        if addr & 1 != 0 {
            // constant pool address
            let obj = self.pool.get(addr >> 1).ok_or_else(|| frame.to_error(ErrorCode::InvalidConstant))?;
            let offset = slot * mem::size_of::<usize>();
            decode_unaligned(obj, offset).map(|x| x.value as usize).ok_or_else(|| {
                frame.to_error(ErrorCode::InvalidAddress)
            })
        } else {
            // heap address, should be aligned
            let slot_addr = addr + slot * mem::size_of::<usize>();
            let slot_ref = unsafe { &*(slot_addr as *const usize) };
            if slot_addr % mem::size_of::<usize>() != 0 || !self.heap.is_inside(slot_ref) {
                return Err(frame.to_error(ErrorCode::InvalidAddress));
            }
            Ok(*slot_ref)
        }
    }

    pub fn store_slot(
        &self,
        addr: usize,
        slot: usize,
        value: usize,
        frame: &StackFrame<'rom, 'heap>,
    ) -> Result<(), RuntimeError<'rom, 'heap>> {
        // can't mutate a constant
        if addr & 1 != 0 { return Err(frame.to_error(ErrorCode::InvalidAddress)); }
        // heap address, should be aligned
        let slot_addr = addr + slot * mem::size_of::<usize>();
        let slot_ref = unsafe { &mut *(slot_addr as *mut usize) };
        if slot_addr % mem::size_of::<usize>() != 0 || !self.heap.is_inside(slot_ref) {
            return Err(frame.to_error(ErrorCode::InvalidAddress));
        }
        *slot_ref = value;
        Ok(())
    }

    pub fn new_object(
        &mut self,
        slots: usize,
        from_stack: usize,
        frame: &mut StackFrame<'rom, 'heap>
    ) -> Result<usize, RuntimeError<'rom, 'heap>> {
        if slots > 64 { return Err(frame.to_error(ErrorCode::InvalidSize)) }
        if from_stack > slots { return Err(frame.to_error(ErrorCode::OutOfBounds)) }
        let obj = self.heap.allocate_array::<usize>(slots).ok_or_else(|| frame.to_error(ErrorCode::OutOfMemory))?;
        let fields = frame.get_n(from_stack)?;
        for i in 0 .. fields.len() { obj[i] = fields[i]; }
        // gross: turn the object into its pointer
        Ok(obj as *mut [usize] as *mut usize as usize)
    }

    pub fn unary(
        &self,
        op: Unary,
        n1: isize,
        frame: &StackFrame<'rom, 'heap>
    ) -> Result<isize, RuntimeError<'rom, 'heap>> {
        match op {
            Unary::Not => Ok(if n1 == 0 { 1 } else { 0 }),
            Unary::Negative => Ok(-n1),
            Unary::BitNot => Ok(!n1),
            _ => Err(frame.to_error(ErrorCode::UnknownOpcode)),
        }
    }

    pub fn binary(
        &self,
        op: Binary,
        n1: isize,
        n2: isize,
        frame: &StackFrame<'rom, 'heap>
    ) -> Result<isize, RuntimeError<'rom, 'heap>> {
        match op {
            Binary::Add => Ok(n1.wrapping_add(n2)),
            Binary::Subtract => Ok(n1.wrapping_sub(n2)),
            Binary::Multiply => Ok(n1.wrapping_mul(n2)),
            Binary::Divide => Ok(n1 / n2),
            Binary::Modulo => Ok(n1 % n2),
            Binary::Equals => Ok(if n1 == n2 { 1 } else { 0 }),
            Binary::LessThan => Ok(if n1 < n2 { 1 } else { 0 }),
            Binary::LessOrEqual => Ok(if n1 <= n2 { 1 } else { 0 }),
            Binary::BitOr => Ok(n1 | n2),
            Binary::BitAnd => Ok(n1 & n2),
            Binary::BitXor => Ok(n1 ^ n2),
            Binary::ShiftLeft => Ok(n1 << n2),
            Binary::ShiftRight => Ok(((n1 as usize) >> n2) as isize),
            Binary::SignShiftRight => Ok(n1 >> n2),
            _ => Err(frame.to_error(ErrorCode::UnknownOpcode)),
        }
    }

}


impl<'rom, 'heap> fmt::Debug for Runtime<'rom, 'heap> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Runtime(pool={:?}, heap={:?})", self.pool, self.heap)
    }
}
