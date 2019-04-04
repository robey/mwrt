use core::{fmt, mem, slice};
use mwgc::Heap;

use crate::error::{ErrorCode, RuntimeError, ToError};

// actually dynamically sized.
#[derive(Default)]
#[repr(C)]
pub struct StackFrame<'rom, 'heap> {
    pub previous: Option<StackFrameMutRef<'rom, 'heap>>,
    pub id: usize,
    // one word for misc:
    pub pc: u16,
    pub sp: u8,
    pub local_count: u8,
    filler: [u8; mem::size_of::<usize>() - 4],
    pub bytecode: &'rom [u8],
    // local storage goes here, then the stack slots
}

pub type StackFrameRef<'rom, 'heap> = &'heap StackFrame<'rom, 'heap>;
pub type StackFrameMutRef<'rom, 'heap> = &'heap mut StackFrame<'rom, 'heap>;

const FRAME_HEADER_WORDS: isize = (mem::size_of::<StackFrame>() / mem::size_of::<usize>()) as isize;


/// Execution state for a code block: a set of local variables and a "stack" for the expression engine
impl<'rom, 'heap> StackFrame<'rom, 'heap> {
    pub fn allocate(
        heap: &mut Heap<'heap>,
        id: usize,
        local_count: u8,
        max_stack: u8,
        bytecode: &'rom [u8],
    ) -> Option<StackFrameMutRef<'rom, 'heap>> {
        let total = (local_count + max_stack) as usize * mem::size_of::<usize>();
        heap.allocate_dynamic_object::<StackFrame>(total).map(|frame| {
            frame.id = id;
            frame.local_count = local_count;
            frame.bytecode = bytecode;
            frame
        })
    }

    pub fn locals(&mut self) -> &'heap mut [usize] {
        let base = self as *mut StackFrame as *mut usize;
        unsafe { slice::from_raw_parts_mut(base.offset(FRAME_HEADER_WORDS), self.local_count as usize) }
    }

    pub fn locals_read(&self) -> &'heap [usize] {
        let base = self as *const StackFrame as *const usize;
        unsafe { slice::from_raw_parts(base.offset(FRAME_HEADER_WORDS), self.local_count as usize) }
    }

    pub fn stack(&mut self, heap: &Heap<'heap>) -> &'heap mut [usize] {
        let base = self as *mut StackFrame as *mut usize;
        let size = heap.size_of(self) / mem::size_of::<usize>();
        let offset = FRAME_HEADER_WORDS + (self.local_count as isize);
        unsafe { slice::from_raw_parts_mut(base.offset(offset), size - (offset as usize)) }
    }

    pub fn stack_read(&self) -> &'heap [usize] {
        let base = self as *const StackFrame as *const usize;
        let offset = FRAME_HEADER_WORDS + (self.local_count as isize);
        unsafe { slice::from_raw_parts(base.offset(offset), self.sp as usize) }
    }

    // the last N things added to the stack
    pub fn stack_from(
        &mut self, index: usize, heap: &Heap<'heap>
    ) -> Result<&'heap mut [usize], RuntimeError<'rom, 'heap>> {
        let stack = self.stack(heap);
        if index > (self.sp as usize) { return Err(self.to_error(ErrorCode::StackUnderflow)) }
        Ok(&mut stack[(self.sp as usize) - index ..])
    }

    pub fn get(&mut self, heap: &mut Heap<'heap>) -> Result<usize, RuntimeError<'rom, 'heap>> {
        let stack = self.stack(heap);
        if self.sp < 1 { return Err(self.to_error(ErrorCode::StackUnderflow)) }
        self.sp -= 1;
        Ok(stack[self.sp as usize])
    }

    pub fn put(&mut self, n: usize, heap: &mut Heap<'heap>) -> Result<(), RuntimeError<'rom, 'heap>> {
        let stack = self.stack(heap);
        if (self.sp as usize) >= stack.len() { return Err(self.to_error(ErrorCode::StackOverflow)) }
        stack[self.sp as usize] = n;
        self.sp += 1;
        Ok(())
    }

    // pub fn put(&mut self, heap: &mut Heap<'heap>, items: &[usize]) -> bool {
    //     let stack = self.stack(heap);
    //     if (self.sp as usize) + items.len() > stack.len() { return false }
    //     for x in items {
    //         stack[self.sp as usize] = *x;
    //         self.sp += 1;
    //     }
    //     true
    // }

    // pub fn put_one(&mut self, heap: &mut Heap<'heap>, item: usize) -> bool {
    //     self.put(heap, &[ item ])
    // }

    pub fn start_locals(&mut self, values: &[usize]) -> Result<(), RuntimeError<'rom, 'heap>> {
        let locals = self.locals();
        if values.len() > locals.len() {
            return Err(self.to_error(ErrorCode::LocalsOverflow));
        }
        for i in 0..values.len() { locals[i] = values[i] }
        Ok(())
    }

    pub fn put_local(&mut self, n: usize, value: usize) -> Result<(), ErrorCode> {
        let locals = self.locals();
        if n >= locals.len() {
            return Err(ErrorCode::LocalsOverflow);
        }
        locals[n] = value;
        Ok(())
    }

}

impl<'rom, 'heap> fmt::Debug for StackFrame<'rom, 'heap> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[frame code={:x} pc={:x} sp={:x}]", self.id, self.pc, self.sp)?;
        if f.alternate() {
            write!(f, "{}", " L={ ")?;
            for i in self.locals_read() { write!(f, "x{:x} ", i)?; }
            write!(f, "{}", "} S={ ")?;
            for i in self.stack_read() { write!(f, "x{:x} ", i)?; }
            write!(f, "{}", "}")?;
        }
        if let Some(prev) = &self.previous {
            write!(f, " -> {:?}", prev)?;
        }
        Ok(())
    }
}

impl<'rom, 'heap> ToError<'rom, 'heap> for StackFrame<'rom, 'heap> {
    // evil trickery:
    // we "know" this is only called to create an error object and will soon
    // become the only heap reference, but rust can't know that.
    fn to_error(&self, code: ErrorCode) -> RuntimeError<'rom, 'heap> {
        let frozen = unsafe { &*(self as *const StackFrame) };
        RuntimeError::new(code, Some(frozen))
    }
}

impl<'rom, 'heap> ToError<'rom, 'heap> for Option<StackFrameMutRef<'rom, 'heap>> {
    fn to_error(&self, code: ErrorCode) -> RuntimeError<'rom, 'heap> {
        let frozen = self.as_ref().map(|f| unsafe { &*(*f as *const StackFrame) });
        RuntimeError::new(code, frozen)
    }
}


#[cfg(test)]
mod tests {
    use core::mem;
    use mwgc::Heap;
    use super::{StackFrame};

    #[test]
    fn locals() {
        let mut data: [u8; 256] = [0; 256];
        let bytecode: [u8; 1] = [ 1 ];
        let mut heap = Heap::from_bytes(&mut data);
        let frame = StackFrame::allocate(&mut heap, 0, 2, 0, &bytecode[..]).unwrap();
        let locals = frame.locals();

        // make sure we allocated enough memory, and that everything is where we expect.
        let heap_used = heap.get_stats().total_bytes - heap.get_stats().free_bytes;
        assert!(heap_used >= mem::size_of::<StackFrame>() + 2 * mem::size_of::<usize>());
        assert_eq!(locals as *mut _ as *mut usize as usize, frame as *mut _ as usize + mem::size_of::<StackFrame>());
        assert!(mem::size_of::<StackFrame>() % mem::size_of::<usize>() == 0);

        assert_eq!(frame.local_count, 2);
        locals[0] = 123456;
        locals[1] = 4;
        assert_eq!(locals[0], 123456);
        assert_eq!(locals[1], 4);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn locals_boundaries() {
        let mut data: [u8; 256] = [0; 256];
        let bytecode: [u8; 1] = [ 1 ];
        println!("data {:?}", data.as_ptr());
        let mut heap = Heap::from_bytes(&mut data);
        let frame = StackFrame::allocate(&mut heap, 0, 2, 0, &bytecode[..]).unwrap();
        frame.locals()[2] = 1;
    }

    #[test]
    fn stack() {
        let mut data: [u8; 256] = [0; 256];
        let bytecode: [u8; 1] = [ 1 ];
        let mut heap = Heap::from_bytes(&mut data);
        let frame = StackFrame::allocate(&mut heap, 0, 2, 2, &bytecode[..]).unwrap();
        let stack = frame.stack(&heap);

        // make sure we allocated enough memory, and that everything is where we expect.
        let heap_used = heap.get_stats().total_bytes - heap.get_stats().free_bytes;
        assert!(heap_used >= mem::size_of::<StackFrame>() + 4 * mem::size_of::<usize>());
        let offset = mem::size_of::<StackFrame>() + 2 * mem::size_of::<usize>();
        assert_eq!(stack as *mut _ as *mut usize as usize, frame as *mut _ as usize + offset);

        assert_eq!(frame.sp, 0);
        stack[0] = 23;
        stack[1] = 19;
        assert_eq!(stack[0], 23);
        assert_eq!(stack[1], 19);
    }
}
