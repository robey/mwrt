use core::{mem, slice};
use mwgc::Heap;

// actually dynamically sized.
#[derive(Default)]
pub struct StackFrame<'heap> {
    previous: Option<&'heap StackFrame<'heap>>,
    size: usize,
    // local storage goes here
}

const FRAME_HEADER_WORDS: isize = (mem::size_of::<StackFrame>() / mem::size_of::<usize>()) as isize;

impl<'heap> StackFrame<'heap> {
    pub fn allocate(
        heap: &mut Heap<'heap>,
        previous: Option<&'heap StackFrame<'heap>>,
        size: usize
    ) -> Option<&'heap mut StackFrame<'heap>> {
        heap.allocate_dynamic_object::<StackFrame>(size * mem::size_of::<usize>()).map(|frame| {
            frame.previous = previous;
            frame.size = size;
            frame
        })
    }

    pub fn locals(&mut self) -> &'heap mut [usize] {
        let base = self as *mut StackFrame as *mut usize;
        unsafe { slice::from_raw_parts_mut(base.offset(FRAME_HEADER_WORDS), self.size) }
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
        println!("data {:?}", data.as_ptr());
        let mut heap = Heap::from_bytes(&mut data);
        let frame = StackFrame::allocate(&mut heap, None, 2).unwrap();
        let locals = frame.locals();

        // make sure we allocated enough memory, and that everything is where we expect.
        let heap_used = heap.get_stats().total_bytes - heap.get_stats().free_bytes;
        assert!(heap_used >= mem::size_of::<StackFrame>() + 2 * mem::size_of::<usize>());
        assert!(locals as *mut _ as *mut usize as usize == frame as *mut _ as usize + mem::size_of::<StackFrame>());

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
        let frame = StackFrame::allocate(&mut heap, None, 2).unwrap();
        frame.locals()[2] = 1;
    }
}
