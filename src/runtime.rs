use core::{fmt, mem};
use mwgc::Heap;

use crate::constant_pool::{ConstantPool};
use crate::disassembler::{decode_next, Instruction};
use crate::error::{ErrorCode, RuntimeError};
use crate::opcode::{Binary, Opcode, Unary};
use crate::stack_frame::{PreviousContext, RuntimeContext};


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
    ) -> Result<Runtime<'rom, 'heap>, RuntimeError> {
        let constant_pool = ConstantPool::new(constant_pool_data);
        let mut heap = Heap::from_bytes(heap_data);
        // just allocate the globals as a heap object
        let globals = heap.allocate_array::<usize>(global_count).ok_or_else(|| {
            RuntimeError::new(ErrorCode::OutOfMemory)
        })?;
        Ok(Runtime { constant_pool, heap, globals, current_time })
    }

    pub fn execute(
        &mut self,
        code_offset: u32,
        args: &[usize],
        results: &mut [usize],
        max_cycles: Option<core::num::NonZeroUsize>,
        deadline: Option<core::num::NonZeroUsize>,
    ) -> Result<usize, RuntimeError> {
        let code_addr = self.constant_pool.addr_from_offset(code_offset);

        let mut context = RuntimeContext::start(&self.constant_pool, &mut self.heap, code_addr).map_err(|e| {
            RuntimeError::new(e)
        })?;

        let mut skip = false;
        let mut cycles = 0;

        context.start_locals(args).map_err(|e| RuntimeError::from(e, &context))?;

        loop {
            if context.frame.pc as usize == context.code.bytecode.len() {
                // ran out of bytecodes? nothing to return.
                return Ok(0);
            }

            // outatime?
            if let (Some(d), Some(t)) = (deadline, self.current_time) {
                if t() >= d.get() {
                    return Err(RuntimeError::from(ErrorCode::TimeExceeded, &context));
                }
            }
            if let Some(m) = max_cycles {
                cycles += 1;
                if cycles > m.get() {
                    return Err(RuntimeError::from(ErrorCode::CyclesExceeded, &context));
                }
            }

            let (instruction, next_pc) =
                decode_next(context.code.bytecode, context.frame.pc).map_err(|e| RuntimeError::from(e, &context))?;
            if skip {
                skip = false;
                context.frame.pc = next_pc;
                continue;
            }

            // println!("-> {} {:#?}", instruction, frame);

            match self.execute_one(instruction, &mut context).map_err(|e| RuntimeError::from(e, &context))? {
                Disposition::Continue => {
                    context.frame.pc = next_pc;
                },
                Disposition::Skip => {
                    context.frame.pc = next_pc;
                    skip = true;
                },
                Disposition::Call(addr, count) => {
                    context.frame.pc = next_pc;
                    context = context.push(&self.constant_pool, &mut self.heap, addr, count).map_err(|e| {
                        RuntimeError::from(e, &context)
                    })?;
                },
                Disposition::Return(count) => {
                    match context.pop(&self.constant_pool, &self.heap, count).map_err(|e| {
                        RuntimeError::from(e, &context)
                    })? {
                        PreviousContext::Done(return_values) => {
                            let n: usize = core::cmp::min(results.len(), return_values.len());
                            results[0..n].copy_from_slice(&return_values[0..n]);
                            return Ok(count);
                        },
                        PreviousContext::Frame(prev) => {
                            context = prev;
                        },
                    }
                },
                Disposition::Jump(new_pc) => {
                    if new_pc as usize >= context.code.bytecode.len() {
                        return Err(RuntimeError::from(ErrorCode::OutOfBounds, &context));
                    }
                    context.frame.pc = new_pc;
                }
            }
        }
    }

    fn execute_one(
        &mut self,
        instruction: Instruction,
        context: &mut RuntimeContext<'rom, 'heap>,
    ) -> Result<Disposition, ErrorCode> {
        match instruction.opcode {
            // zero immediates:

            Opcode::Break => {
                return Err(ErrorCode::Break);
            },
            Opcode::Nop => {
                // nothing
            },
            Opcode::Dup => {
                let v = context.get()?;
                context.put(v)?;
                context.put(v)?;
            },
            Opcode::Drop => {
                context.get()?;
            },
            Opcode::Call => {
                let count = context.get()?;
                let addr = context.get()?;
                return Ok(Disposition::Call(addr, count));
            },
            Opcode::Return => {
                let count = context.get()?;
                return Ok(Disposition::Return(count));
            },
            Opcode::New => {
                let fill = context.get()?;
                let slots = context.get()?;
                let obj = self.new_object(slots, fill, context)?;
                context.put(obj)?;
            },
            Opcode::Size => {
                let addr = context.get()?;
                context.put(self.object_size(addr)?)?;
            },
            Opcode::LoadSlot => {
                let slot = context.get()?;
                let v = self.load_slot(context.get()?, slot)?;
                context.put(v)?;
            },
            Opcode::StoreSlot => {
                let v = context.get()?;
                let slot = context.get()?;
                self.store_slot(context.get()?, slot, v)?;
            },
            Opcode::If => {
                if context.get()? == 0 { return Ok(Disposition::Skip); }
            },

            // one immediate:

            Opcode::Immediate => {
                context.put(instruction.n1 as usize)?;
            },
            Opcode::Constant => {
                context.put(self.constant_pool.addr_from_offset(instruction.n1 as u32))?;
            },
            Opcode::LoadSlotN => {
                let v = self.load_slot(context.get()?, instruction.n1 as usize)?;
                context.put(v)?;
            },
            Opcode::StoreSlotN => {
                let v = context.get()?;
                self.store_slot(context.get()?, instruction.n1 as usize, v)?;
            },
            Opcode::LoadLocalN => {
                let locals = context.locals();
                let n = instruction.n1 as usize;
                if n >= locals.len() { return Err(ErrorCode::OutOfBounds) }
                context.put(locals[n])?;
            },
            Opcode::StoreLocalN => {
                let locals = context.locals_mut();
                let n = instruction.n1 as usize;
                if n >= locals.len() { return Err(ErrorCode::OutOfBounds) }
                locals[n] = context.get()?;
            },
            Opcode::LoadGlobalN => {
                let n = instruction.n1 as usize;
                if n >= self.globals.len() { return Err(ErrorCode::OutOfBounds) }
                context.put(self.globals[n])?;
            },
            Opcode::StoreGlobalN => {
                let n = instruction.n1 as usize;
                if n >= self.globals.len() { return Err(ErrorCode::OutOfBounds) }
                self.globals[n] = context.get()?;
            },
            Opcode::Unary => {
                let v = context.get()?;
                let op = Unary::from_usize(instruction.n1 as usize);
                context.put(self.unary(op, v as isize)? as usize)?;
            },
            Opcode::Binary => {
                let v2 = context.get()?;
                let v1 = context.get()?;
                let op = Binary::from_usize(instruction.n1 as usize);
                context.put(self.binary(op, v1 as isize, v2 as isize)? as usize)?;
            },
            Opcode::CallN => {
                let addr = context.get()?;
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
                let obj = self.new_object(instruction.n1 as usize, instruction.n2 as usize, context)?;
                context.put(obj)?;
            },

            _ => {
                return Err(ErrorCode::UnknownOpcode);
            }
        }

        Ok(Disposition::Continue)
    }

    pub fn object_size(
        &self,
        addr: usize,
    ) -> Result<usize, ErrorCode> {
        if self.heap.is_ptr_inside(addr as *const usize) {
            Ok(self.heap.size_of_ptr(addr as *const usize) / mem::size_of::<usize>())
        } else {
            // only valid for heap addresses
            Err(ErrorCode::InvalidAddress)
        }
    }

    pub fn load_slot(
        &self,
        addr: usize,
        slot: usize,
    ) -> Result<usize, ErrorCode> {
        // must be aligned
        let slot_addr = addr + slot * mem::size_of::<usize>();
        if slot_addr % mem::size_of::<usize>() != 0 { return Err(ErrorCode::Unaligned) }
        let slot_ptr = slot_addr as *const usize;
        let slot = self.constant_pool.safe_ref(slot_ptr).or_else(|| {
            self.heap.safe_ref(slot_ptr)
        }).ok_or(ErrorCode::InvalidAddress)?;
        Ok(*slot)
    }

    pub fn store_slot(
        &self,
        addr: usize,
        slot: usize,
        value: usize,
    ) -> Result<(), ErrorCode> {
        // must be heap address, and aligned
        let slot_addr = addr + slot * mem::size_of::<usize>();
        if slot_addr % mem::size_of::<usize>() != 0 { return Err(ErrorCode::Unaligned) }
        let slot_ptr = slot_addr as *mut usize;
        let obj = self.heap.safe_ref_mut(slot_ptr).ok_or(ErrorCode::InvalidAddress)?;
        *obj = value;
        Ok(())
    }

    pub fn new_object(
        &mut self,
        slots: usize,
        from_stack: usize,
        context: &mut RuntimeContext<'rom, 'heap>
    ) -> Result<usize, ErrorCode> {
        if slots > 64 { return Err(ErrorCode::InvalidSize) }
        if from_stack > slots { return Err(ErrorCode::OutOfBounds) }
        let obj = self.heap.allocate_array::<usize>(slots).ok_or(ErrorCode::OutOfMemory)?;
        let fields = context.get_n(from_stack)?;
        for i in 0 .. fields.len() { obj[i] = fields[i]; }
        // gross: turn the object into its pointer
        Ok(obj as *mut [usize] as *mut usize as usize)
    }

    pub fn unary(
        &self,
        op: Unary,
        n1: isize,
    ) -> Result<isize, ErrorCode> {
        match op {
            Unary::Not => Ok(if n1 == 0 { 1 } else { 0 }),
            Unary::Negative => Ok(-n1),
            Unary::BitNot => Ok(!n1),
            _ => Err(ErrorCode::UnknownOpcode),
        }
    }

    pub fn binary(
        &self,
        op: Binary,
        n1: isize,
        n2: isize,
    ) -> Result<isize, ErrorCode> {
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
            _ => Err(ErrorCode::UnknownOpcode),
        }
    }
}


impl<'rom, 'heap> fmt::Debug for Runtime<'rom, 'heap> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Runtime(pool={:?}, heap={:?})", self.constant_pool.data, self.heap)
    }
}
