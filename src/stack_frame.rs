use core::{fmt, mem, slice};
use mwgc::Heap;

use crate::constant_pool::{Code, ConstantPool};
use crate::error::{ErrorCode, RuntimeError};

/// A stack frame as it exists on the runtime's heap, in a linked list back
/// to the starting frame.
/// It's actually dynamically sized, with a header (this struct), which
/// should be either 2 (64-bit) or 3 (32-bit) words, followed by a set of
/// local variables and a "stack" for the expression engine.
#[derive(Default)]
#[repr(C)]
pub struct StackFrame {
    // stored as a pointer into the heap:
    pub up_frame: usize,
    // offset into the constant pool:
    pub code_offset: u32,
    // 32 bits of other metadata:
    pub pc: u16,
    pub sp: u8,
    unused1: u8,
    // local storage goes here, then the stack slots
}

/// Everything in the stack frame, plus the decoded Code object, which we can
/// reconstruct each time we call or return from a function, so we don't need
/// to waste heap space on it.
pub struct RuntimeContext<'rom, 'heap> {
    pub frame: &'heap mut StackFrame,
    pub code: Code<'rom>,
}

pub enum PreviousContext<'rom, 'heap> {
    Frame(RuntimeContext<'rom, 'heap>),
    Done(&'heap [usize]),
}

impl<'rom, 'heap> RuntimeContext<'rom, 'heap> {
    fn new(
        constant_pool: &ConstantPool<'rom>,
        heap: &mut Heap<'heap>,
        code_addr: usize,
        up_frame: usize,
    ) -> Result<RuntimeContext<'rom, 'heap>, ErrorCode> {
        let code = constant_pool.get_code(code_addr)?;
        let total = (code.local_count + code.max_stack) as usize * mem::size_of::<usize>();
        let frame = heap.allocate_dynamic_object::<StackFrame>(total).ok_or(ErrorCode::OutOfMemory)?;
        frame.up_frame = up_frame;
        frame.code_offset = constant_pool.offset_from_addr(code_addr);
        Ok(RuntimeContext { frame, code })
    }

    /// Allocate a new stack frame with no previous frame (this is the starting frame).
    pub fn start(
        constant_pool: &ConstantPool<'rom>,
        heap: &mut Heap<'heap>,
        code_addr: usize,
    ) -> Result<RuntimeContext<'rom, 'heap>, ErrorCode> {
        RuntimeContext::new(constant_pool, heap, code_addr, core::ptr::null::<StackFrame>() as usize)
    }

    /// Allocate a new stack frame that links back to this one.
    pub fn push(
        &mut self,
        constant_pool: &ConstantPool<'rom>,
        heap: &mut Heap<'heap>,
        code_addr: usize,
        arg_count: usize,
    ) -> Result<RuntimeContext<'rom, 'heap>, ErrorCode> {
        let args = self.get_n(arg_count)?;
        let mut next = RuntimeContext::new(constant_pool, heap, code_addr, self.frame as *const StackFrame as usize)?;
        next.start_locals(args)?;
        Ok(next)
    }

    /// Drop this stack frame and return the previous one, if there was one.
    pub fn pop(
        &mut self,
        constant_pool: &ConstantPool<'rom>,
        heap: &Heap<'heap>,
        return_count: usize,
    ) -> Result<PreviousContext<'rom, 'heap>, ErrorCode> {
        let return_values = self.get_n(return_count)?;

        let ptr = self.frame.up_frame as *mut StackFrame;
        if ptr.is_null() { return Ok(PreviousContext::Done(return_values)) }

        // none of these should error out, since they worked on the way in
        let frame = heap.safe_ref_mut(ptr).ok_or(ErrorCode::InvalidAddress)?;
        let code_addr = constant_pool.addr_from_offset(frame.code_offset);
        let code = constant_pool.get_code(code_addr)?;

        let mut prev = RuntimeContext { frame, code };
        prev.put_n(return_values)?;
        Ok(PreviousContext::Frame(prev))
    }

    pub fn locals_mut(&mut self) -> &'heap mut [usize] {
        let base = self.frame as *mut StackFrame as *mut usize;
        unsafe { slice::from_raw_parts_mut(base.offset(FRAME_HEADER_WORDS), self.code.local_count as usize) }
    }

    pub fn locals(&self) -> &'heap [usize] {
        let base = self.frame as *const StackFrame as *const usize;
        unsafe { slice::from_raw_parts(base.offset(FRAME_HEADER_WORDS), self.code.local_count as usize) }
    }

    pub fn stack_mut(&mut self) -> &'heap mut [usize] {
        let base = self.frame as *mut StackFrame as *mut usize;
        let offset = FRAME_HEADER_WORDS + (self.code.local_count as isize);
        unsafe { slice::from_raw_parts_mut(base.offset(offset), self.code.max_stack as usize) }
    }

    pub fn stack(&self) -> &'heap [usize] {
        let base = self.frame as *const StackFrame as *const usize;
        let offset = FRAME_HEADER_WORDS + (self.code.local_count as isize);
        unsafe { slice::from_raw_parts(base.offset(offset), self.frame.sp as usize) }
    }

    pub fn get(&mut self) -> Result<usize, ErrorCode> {
        let stack = self.stack();
        if self.frame.sp < 1 { return Err(ErrorCode::StackUnderflow) }
        self.frame.sp -= 1;
        Ok(stack[self.frame.sp as usize])
    }

    // the last N things added to the stack
    pub fn get_n(&mut self, n: usize) -> Result<&'heap [usize], ErrorCode> {
        let stack = self.stack();
        if self.frame.sp < (n as u8) { return Err(ErrorCode::StackUnderflow) }
        self.frame.sp -= n as u8;
        let start = self.frame.sp as usize;
        Ok(&stack[start .. start + n])
    }

    pub fn put(&mut self, n: usize) -> Result<(), ErrorCode> {
        let stack = self.stack_mut();
        if (self.frame.sp as usize) >= stack.len() { return Err(ErrorCode::StackOverflow) }
        stack[self.frame.sp as usize] = n;
        self.frame.sp += 1;
        Ok(())
    }

    pub fn put_n(&mut self, items: &'heap [usize]) -> Result<(), ErrorCode> {
        for item in items.iter() { self.put(*item)? }
        Ok(())
    }

    pub fn start_locals(&mut self, values: &[usize]) -> Result<(), ErrorCode> {
        let locals = self.locals_mut();
        if values.len() > locals.len() { return Err(ErrorCode::LocalsOverflow) }
        for i in 0..values.len() { locals[i] = values[i] }
        Ok(())
    }

    pub fn get_local(&mut self, n: usize) -> Result<usize, ErrorCode> {
        let locals = self.locals();
        if n >= locals.len() {
            return Err(ErrorCode::LocalsOverflow);
        }
        Ok(locals[n])
    }

    pub fn put_local(&mut self, n: usize, value: usize) -> Result<(), ErrorCode> {
        let locals = self.locals_mut();
        if n >= locals.len() {
            return Err(ErrorCode::LocalsOverflow);
        }
        locals[n] = value;
        Ok(())
    }

    pub fn to_error(&self, code: ErrorCode) -> RuntimeError {
        RuntimeError::from(code, self)
    }
}

pub const FRAME_HEADER_WORDS: isize = (mem::size_of::<StackFrame>() / mem::size_of::<usize>()) as isize;


impl fmt::Debug for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[frame code={:x} pc={:x} sp={:x}]", self.code_offset, self.pc, self.sp)?;
        if let Some(prev) = unsafe { (self.up_frame as *const StackFrame).as_ref() } {
            write!(f, " -> {:?}", prev)?;
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use core::mem;
    use mwgc::Heap;
    use crate::constant_pool::ConstantPool;
    use super::{FRAME_HEADER_WORDS, RuntimeContext, StackFrame};

    #[test]
    fn locals() {
        let mut data: [u8; 256] = [0; 256];
        let mut heap = Heap::from_bytes(&mut data);
        let pool = ConstantPool::new(&[ 2, 0, 1, 0, 0 ]);
        let mut context = RuntimeContext::start(&pool, &mut heap, pool.addr_from_offset(0)).unwrap();
        let locals = context.locals_mut();

        // make sure we allocated enough memory, and that everything is where we expect.
        let heap_used = heap.get_stats().total_bytes - heap.get_stats().free_bytes;
        assert!(heap_used >= mem::size_of::<StackFrame>() + 2 * mem::size_of::<usize>());
        assert_eq!(
            locals as *mut _ as *mut usize as usize,
            context.frame as *mut _ as usize + mem::size_of::<StackFrame>()
        );
        assert!(mem::size_of::<StackFrame>() % mem::size_of::<usize>() == 0);

        assert_eq!(context.code.local_count, 2);
        locals[0] = 123456;
        locals[1] = 4;
        assert_eq!(locals[0], 123456);
        assert_eq!(locals[1], 4);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn locals_boundaries() {
        let mut data: [u8; 256] = [0; 256];
        let mut heap = Heap::from_bytes(&mut data);
        let pool = ConstantPool::new(&[ 2, 0, 1, 0, 0 ]);
        let mut context = RuntimeContext::start(&pool, &mut heap, pool.addr_from_offset(0)).unwrap();
        context.locals_mut()[2] = 1;
    }

    #[test]
    fn stack() {
        let mut data: [u8; 256] = [0; 256];
        let mut heap = Heap::from_bytes(&mut data);
        let pool = ConstantPool::new(&[ 2, 2, 1, 0, 0 ]);
        let mut context = RuntimeContext::start(&pool, &mut heap, pool.addr_from_offset(0)).unwrap();
        let stack = context.stack_mut();

        // make sure we allocated enough memory, and that everything is where we expect.
        let heap_used = heap.get_stats().total_bytes - heap.get_stats().free_bytes;
        assert!(heap_used >= mem::size_of::<StackFrame>() + 4 * mem::size_of::<usize>());
        let offset = mem::size_of::<StackFrame>() + 2 * mem::size_of::<usize>();
        assert_eq!(stack as *mut _ as *mut usize as usize, context.frame as *mut _ as usize + offset);

        assert_eq!(context.frame.sp, 0);
        stack[0] = 23;
        stack[1] = 19;
        assert_eq!(stack[0], 23);
        assert_eq!(stack[1], 19);
    }

    #[test]
    fn allocation_size() {
        assert_eq!(FRAME_HEADER_WORDS, if mem::size_of::<usize>() == 4 { 3 } else { 2 })
    }
}
