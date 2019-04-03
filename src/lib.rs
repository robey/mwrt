mod constant_pool;
mod decode_int;
mod error;
mod runtime;
mod stack_frame;

pub use error::{ErrorCode};
pub use runtime::{Runtime};

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
