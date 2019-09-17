use core::{fmt, mem};
use mwgc::Heap;

use crate::constant_pool::{ConstantPool};
use crate::disassembler::{decode_next, Instruction};
use crate::error::{ErrorCode, RuntimeError, ToError};
use crate::opcode::{Binary, Opcode, Unary};
use crate::stack_frame::{StackFrame, StackFrameMutRef};


// pub struct RuntimeOptions {
//     pub global_count: usize,
//     pub current_time: Option<fn() -> usize>,
// }


// what to do after executing a bytecode
#[derive(Debug)]
enum Disposition {
    Continue,       // keep going, possibly across a jump
    Skip,           // skip next instruction
    Call(usize, usize),
    Return(usize),
    Jump(u16),
}

// runtime context holds a stack frame reference, and a bytecode slice
// reference, to save space in the stack frame.
// (the bytecode slice can be recomputed from the stack frame when we
// return.)
struct RuntimeContext<'rom, 'heap> {
    pub frame: StackFrameMutRef<'rom, 'heap>,
    pub bytecode: &'rom [u8],
}


pub struct Runtime<'rom, 'heap> {
    constant_pool: ConstantPool<'rom>,
    heap: Heap<'heap>,
    globals: &'heap mut [usize],
    current_time: Option<fn() -> usize>,
}

impl<'rom, 'heap> Runtime<'rom, 'heap> {
    pub fn new(
        constant_pool_data: &'rom [u8],
        heap_data: &'heap mut [u8],
        global_count: usize,
        current_time: Option<fn() -> usize>,
    ) -> Result<Runtime<'rom, 'heap>, RuntimeError<'rom, 'heap>> {
        let constant_pool = ConstantPool::new(constant_pool_data);
        let mut heap = Heap::from_bytes(heap_data);
        // just allocate the globals as a heap object
        let globals = heap.allocate_array::<usize>(global_count).ok_or_else(|| {
            RuntimeError::new(ErrorCode::OutOfMemory, None)
        })?;
        Ok(Runtime { constant_pool, heap, globals, current_time })
    }

    pub fn execute(
        &mut self,
        offset: usize,
        args: &[usize],
        results: &mut [usize],
        max_cycles: Option<core::num::NonZeroUsize>,
        deadline: Option<core::num::NonZeroUsize>,
    ) -> Result<usize, RuntimeError<'rom, 'heap>> {
        let mut context = self.frame_from_code(self.constant_pool.addr_from_offset(offset), None, args)?;
        let mut skip = false;
        let mut cycles = 0;

        loop {
            if context.frame.pc as usize == context.bytecode.len() {
                // ran out of bytecodes? nothing to return.
                return Ok(0);
            }

            // outatime?
            if let (Some(d), Some(t)) = (deadline, self.current_time) {
                if t() >= d.get() {
                    return Err(context.frame.to_error(ErrorCode::TimeExceeded));
                }
            }
            if let Some(m) = max_cycles {
                cycles += 1;
                if cycles > m.get() {
                    return Err(context.frame.to_error(ErrorCode::CyclesExceeded));
                }
            }

            let (instruction, next_pc) =
                decode_next(context.bytecode, context.frame.pc).map_err(|e| context.frame.to_error(e))?;
            if skip {
                skip = false;
                context.frame.pc = next_pc;
                continue;
            }

            // println!("-> {} {:#?}", instruction, frame);

            match self.execute_one(instruction, context.frame)? {
                Disposition::Continue => {
                    context.frame.pc = next_pc;
                },
                Disposition::Skip => {
                    context.frame.pc = next_pc;
                    skip = true;
                },
                Disposition::Call(addr, count) => {
                    context.frame.pc = next_pc;
                    let args = context.frame.get_n(count)?;
                    context = self.frame_from_code(addr, Some(context.frame), args)?;
                },
                Disposition::Return(count) => {
                    let stack_results = context.frame.get_n(count)?;
                    let previous = context.frame.previous.take();
                    match previous {
                        None => {
                            let n: usize = if count < results.len() { count } else { results.len() };
                            for i in 0 .. n { results[i] = stack_results[i] }
                            return Ok(count);
                        },
                        Some(previous) => {
                            for i in 0 .. count { previous.put(stack_results[i])?; }
                            let code_addr = self.constant_pool.addr_from_offset(previous.code_offset);
                            let code = self.constant_pool.get_code(code_addr).map_err(|e| previous.to_error(e))?;
                            context = RuntimeContext { frame: previous, bytecode: code.bytecode };
                        }
                    }
                },
                Disposition::Jump(new_pc) => {
                    if new_pc as usize >= context.bytecode.len() {
                        return Err(context.frame.to_error(ErrorCode::OutOfBounds));
                    }
                    context.frame.pc = new_pc;
                }
            }
        }
    }

    fn execute_one(
        &mut self,
        instruction: Instruction,
        frame: &mut StackFrame<'rom, 'heap>
    ) -> Result<Disposition, RuntimeError<'rom, 'heap>> {
        match instruction.opcode {
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
            Opcode::Drop => {
                frame.get()?;
            },
            Opcode::Call => {
                let count = frame.get()?;
                let addr = frame.get()?;
                return Ok(Disposition::Call(addr, count));
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
            Opcode::LoadSlot => {
                let slot = frame.get()?;
                let v = self.load_slot(frame.get()?, slot, frame)?;
                frame.put(v)?;
            },
            Opcode::StoreSlot => {
                let v = frame.get()?;
                let slot = frame.get()?;
                self.store_slot(frame.get()?, slot, v, frame)?;
            },
            Opcode::If => {
                if frame.get()? == 0 { return Ok(Disposition::Skip); }
            },

            // one immediate:

            Opcode::Immediate => {
                frame.put(instruction.n1 as usize)?;
            },
            Opcode::Constant => {
                frame.put(self.constant_pool.addr_from_offset(instruction.n1 as usize))?;
            },
            Opcode::LoadSlotN => {
                let v = self.load_slot(frame.get()?, instruction.n1 as usize, frame)?;
                frame.put(v)?;
            },
            Opcode::StoreSlotN => {
                let v = frame.get()?;
                self.store_slot(frame.get()?, instruction.n1 as usize, v, frame)?;
            },
            Opcode::LoadLocalN => {
                let locals = frame.locals();
                let n = instruction.n1 as usize;
                if n >= locals.len() { return Err(frame.to_error(ErrorCode::OutOfBounds)) }
                frame.put(locals[n])?;
            },
            Opcode::StoreLocalN => {
                let locals = frame.locals_mut();
                let n = instruction.n1 as usize;
                if n >= locals.len() { return Err(frame.to_error(ErrorCode::OutOfBounds)) }
                locals[n] = frame.get()?;
            },
            Opcode::LoadGlobalN => {
                let n = instruction.n1 as usize;
                if n >= self.globals.len() { return Err(frame.to_error(ErrorCode::OutOfBounds)) }
                frame.put(self.globals[n])?;
            },
            Opcode::StoreGlobalN => {
                let n = instruction.n1 as usize;
                if n >= self.globals.len() { return Err(frame.to_error(ErrorCode::OutOfBounds)) }
                self.globals[n] = frame.get()?;
            },
            Opcode::Unary => {
                let v = frame.get()?;
                let op = Unary::from_usize(instruction.n1 as usize);
                frame.put(self.unary(op, v as isize, frame)? as usize)?;
            },
            Opcode::Binary => {
                let v2 = frame.get()?;
                let v1 = frame.get()?;
                let op = Binary::from_usize(instruction.n1 as usize);
                frame.put(self.binary(op, v1 as isize, v2 as isize, frame)? as usize)?;
            },
            Opcode::CallN => {
                let addr = frame.get()?;
                return Ok(Disposition::Call(addr, instruction.n1 as usize));
            },
            Opcode::ReturnN => {
                return Ok(Disposition::Return(instruction.n1 as usize));
            },
            Opcode::Jump => {
                return Ok(Disposition::Jump(instruction.n1 as u16));
            },

            // two immediates:

            Opcode::NewNN => {
                let obj = self.new_object(instruction.n1 as usize, instruction.n2 as usize, frame)?;
                frame.put(obj)?;
            },

            _ => {
                return Err(frame.to_error(ErrorCode::UnknownOpcode));
            }
        }

        Ok(Disposition::Continue)
    }

    // look up a code object in the constant pool, allocate a new stack frame
    // for it, and load the arguments into locals.
    // return the new stack frame, and a slice representing the bytecode.
    fn frame_from_code(
        &mut self,
        addr: usize,
        prev_frame: Option<StackFrameMutRef<'rom, 'heap>>,
        args: &[usize]
    ) -> Result<RuntimeContext<'rom, 'heap>, RuntimeError<'rom, 'heap>> {
        let code = self.constant_pool.get_code(addr).map_err(|e| prev_frame.to_error(e))?;
        let mut frame = StackFrame::allocate(
            &mut self.heap, self.constant_pool.offset_from_addr(addr), code.local_count, code.max_stack
        ).ok_or(
            prev_frame.to_error(ErrorCode::OutOfMemory)
        )?;

        frame.previous = prev_frame;
        frame.start_locals(args)?;
        Ok(RuntimeContext { frame, bytecode: code.bytecode })
    }

     pub fn object_size(
        &self,
        addr: usize,
        frame: &StackFrame<'rom, 'heap>
    ) -> Result<usize, RuntimeError<'rom, 'heap>> {
        if self.heap.is_ptr_inside(addr as *const usize) {
            Ok(self.heap.size_of_ptr(addr as *const usize) / mem::size_of::<usize>())
        } else {
            // only valid for heap addresses
            Err(frame.to_error(ErrorCode::InvalidAddress))
        }
    }

    pub fn load_slot(
        &self,
        addr: usize,
        slot: usize,
        frame: &StackFrame<'rom, 'heap>
    ) -> Result<usize, RuntimeError<'rom, 'heap>> {
        // must be aligned
        let slot_addr = addr + slot * mem::size_of::<usize>();
        if slot_addr % mem::size_of::<usize>() != 0 { return Err(frame.to_error(ErrorCode::Unaligned)) }
        let slot_ptr = slot_addr as *const usize;
        let slot = self.constant_pool.safe_ref(slot_ptr).or_else(|| {
            self.heap.safe_ref(slot_ptr)
        }).ok_or(frame.to_error(ErrorCode::InvalidAddress))?;
        Ok(*slot)
    }

    pub fn store_slot(
        &self,
        addr: usize,
        slot: usize,
        value: usize,
        frame: &StackFrame<'rom, 'heap>,
    ) -> Result<(), RuntimeError<'rom, 'heap>> {
        // must be heap address, and aligned
        let slot_addr = addr + slot * mem::size_of::<usize>();
        if slot_addr % mem::size_of::<usize>() != 0 { return Err(frame.to_error(ErrorCode::Unaligned)) }
        let slot_ptr = slot_addr as *mut usize;
        let obj = self.heap.safe_ref_mut(slot_ptr).ok_or(frame.to_error(ErrorCode::InvalidAddress))?;
        *obj = value;
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
        write!(f, "Runtime(pool={:?}, heap={:?})", self.constant_pool.data, self.heap)
    }
}
