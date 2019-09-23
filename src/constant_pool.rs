use core::mem;

use crate::error::{ErrorCode};


/// Decoded code block metadata from the constant pool. This is allocated on
/// our (runtime) stack, so it needs to be relatively compact, but it won't
/// be on the heap like the stack frame.
pub struct Code<'rom> {
    pub local_count: u8,
    pub max_stack: u8,
    pub bytecode: &'rom [u8],
}

impl<'rom> Code<'rom> {
    pub fn new(local_count: u8, max_stack: u8, bytecode: &'rom [u8]) -> Code<'rom> {
        Code { local_count, max_stack, bytecode }
    }
}


/// Wrapper for a `&'rom [u8]` that provides functions to safely access
/// small bits of its internals.
pub struct ConstantPool<'rom> {
    pub data: &'rom [u8],
}

impl<'rom> ConstantPool<'rom> {
    pub fn new(data: &'rom [u8]) -> ConstantPool<'rom> {
        ConstantPool { data }
    }

    // offsets are always shifted 2 bits right
    pub fn addr_from_offset(&self, offset: u32) -> usize {
        (self.data.as_ptr() as usize) + ((offset as usize) << 2)
    }

    pub fn offset_from_addr(&self, addr: usize) -> u32 {
        ((addr - (self.data.as_ptr() as usize)) >> 2) as u32
    }

    /// If this address points to a part of the constant pool that seems to
    /// represent a code block, parse and return it.
    pub fn get_code(&self, addr: usize) -> Result<Code<'rom>, ErrorCode> {
        // header must be present and readable, to start with
        let header = self.safe_ref(addr as *const [u8; 4]).ok_or(ErrorCode::InvalidAddress)?;
        if header[0] > 63 || header[1] > 63 { return Err(ErrorCode::InvalidCodeObject) }
        let local_count = header[0];
        let max_stack = header[1];
        // code block must also be valid
        let len = (header[2] as usize) + ((header[3] as usize) << 8);
        let bytecode = self.safe_slice((addr + 4) as *const u8, len).ok_or(ErrorCode::InvalidAddress)?;
        Ok(Code { local_count, max_stack, bytecode })
    }

    /// Turn a pointer into a reference if it's safely within the constant pool.
    pub fn safe_ref<T>(&self, ptr: *const T) -> Option<&'rom T> {
        if self.is_in_constant_pool(ptr) { Some(unsafe { &*ptr }) } else { None }
    }

    /// Turn a pointer and length into a slice if it's safely within the constant pool.
    pub fn safe_slice<T>(&self, ptr: *const T, len: usize) -> Option<&'rom [T]> {
        if self.is_in_constant_pool_range(ptr as usize, len * mem::size_of::<T>()) {
            Some(unsafe { core::slice::from_raw_parts(ptr, len) })
        } else {
            None
        }
    }

    fn is_in_constant_pool<T>(&self, obj: *const T) -> bool {
        self.is_in_constant_pool_range(obj as usize, mem::size_of::<T>())
    }

    fn is_in_constant_pool_range(&self, addr: usize, len: usize) -> bool {
        let pool = self.data.as_ptr() as usize;
        addr >= pool && addr + len <= pool + self.data.len()
    }
}
