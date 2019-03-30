use core::{mem, slice};
use mwgc::Heap;

// actually dynamically sized.
#[derive(Default)]
pub struct StackFrame<'heap> {
    pub previous: Option<&'heap StackFrame<'heap>>,
    metadata: usize,    // local count (u8), and current stack position (u8)
    // local storage goes here, then the stack slots
}

const FRAME_HEADER_WORDS: isize = (mem::size_of::<StackFrame>() / mem::size_of::<usize>()) as isize;

/// Execution state for a code block: a set of local variables and a "stack" for the expression engine
impl<'heap> StackFrame<'heap> {
    pub fn allocate(
        heap: &mut Heap<'heap>,
        previous: Option<&'heap StackFrame<'heap>>,
        local_count: usize,
        max_stack: usize,
    ) -> Option<&'heap mut StackFrame<'heap>> {
        let total = (local_count + max_stack) * mem::size_of::<usize>();
        heap.allocate_dynamic_object::<StackFrame>(total).map(|frame| {
            frame.previous = previous;
            frame.metadata = local_count;
            frame
        })
    }

    pub fn local_count(&self) -> usize {
        self.metadata & 0xff
    }

    pub fn get_sp(&self) -> usize {
        (self.metadata >> 8) & 0xff
    }

    pub fn set_sp(&mut self, sp: usize) {
        self.metadata = (self.metadata & 0xff) | ((sp & 0xff) << 8);
    }

    pub fn locals(&mut self) -> &'heap mut [usize] {
        let base = self as *mut StackFrame as *mut usize;
        unsafe { slice::from_raw_parts_mut(base.offset(FRAME_HEADER_WORDS), self.local_count()) }
    }

    pub fn stack(&mut self, heap: &'heap Heap<'heap>) -> &'heap mut [usize] {
        let base = self as *mut StackFrame as *mut usize;
        let size = heap.size_of(self) / mem::size_of::<usize>();
        let offset = FRAME_HEADER_WORDS + (self.local_count() as isize);
        unsafe { slice::from_raw_parts_mut(base.offset(offset), size - (offset as usize)) }
    }
}


#[cfg(test)]
mod tests {
    use core::mem;
    use crate::Heap;
    use super::{StackFrame};

    #[test]
    fn locals() {
        let mut data: [u8; 256] = [0; 256];
        let mut heap = Heap::from_bytes(&mut data);
        let frame = StackFrame::allocate(&mut heap, None, 2, 0).unwrap();
        let locals = frame.locals();

        // make sure we allocated enough memory, and that everything is where we expect.
        let heap_used = heap.get_stats().total_bytes - heap.get_stats().free_bytes;
        assert!(heap_used >= mem::size_of::<StackFrame>() + 2 * mem::size_of::<usize>());
        assert_eq!(locals as *mut _ as *mut usize as usize, frame as *mut _ as usize + mem::size_of::<StackFrame>());

        assert_eq!(frame.local_count(), 2);
        locals[0] = 123456;
        locals[1] = 4;
        assert_eq!(locals[0], 123456);
        assert_eq!(locals[1], 4);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn locals_boundaries() {
        let mut data: [u8; 256] = [0; 256];
        println!("data {:?}", data.as_ptr());
        let mut heap = Heap::from_bytes(&mut data);
        let frame = StackFrame::allocate(&mut heap, None, 2, 0).unwrap();
        frame.locals()[2] = 1;
    }

    #[test]
    fn stack() {
        let mut data: [u8; 256] = [0; 256];
        let mut heap = Heap::from_bytes(&mut data);
        let frame = StackFrame::allocate(&mut heap, None, 2, 2).unwrap();
        let locals = frame.locals();
        let stack = frame.stack(&heap);

        // make sure we allocated enough memory, and that everything is where we expect.
        let heap_used = heap.get_stats().total_bytes - heap.get_stats().free_bytes;
        assert!(heap_used >= mem::size_of::<StackFrame>() + 4 * mem::size_of::<usize>());
        let offset = mem::size_of::<StackFrame>() + 2 * mem::size_of::<usize>();
        assert_eq!(stack as *mut _ as *mut usize as usize, frame as *mut _ as usize + offset);

        assert_eq!(frame.get_sp(), 0);
        stack[0] = 23;
        stack[1] = 19;
        frame.set_sp(2);
        assert_eq!(frame.get_sp(), 2);
        assert_eq!(stack[0], 23);
        assert_eq!(stack[1], 19);
    }
}
