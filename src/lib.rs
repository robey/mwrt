use mwgc::Heap;

mod constant_pool;
mod runtime;
mod stack_frame;
mod zigzag;

pub use runtime::{ErrorCode, Runtime, RuntimeError};

#[cfg(test)]
mod tests {
    use crate::stack_frame::StackFrame;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
        println!("{}", core::mem::size_of::<StackFrame>());
        // assert!(false);
    }
}
