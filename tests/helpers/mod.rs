// helpers to make a runtime

use core::mem;
use mwrt::{Runtime, RuntimeError};

const DEFAULT_GLOBALS: usize = 2;
const DEFAULT_LOCALS: usize = 8;
const DEFAULT_STACK: usize = 8;

pub struct Bytes {
    data: [u8; 128],
    index: usize,
}

impl Bytes {
    pub fn code(local_count: usize, stack_count: usize, codes: &[&[u8]]) -> Bytes {
        assert!(local_count < 128);
        assert!(stack_count < 128);
        let len = codes.iter().map(|code| code.len()).fold(0, |a, b| a + b);
        let mut b = Bytes { data: [0; 128], index: 4 };
        b.data[0] = local_count as u8;
        b.data[1] = stack_count as u8;
        b.data[2] = (len & 0xff) as u8;
        b.data[3] = ((len >> 8) & 0xff) as u8;
        for c in codes { b.add(c) }
        b
    }

    pub fn basic_code(codes: &[&[u8]]) -> Bytes {
        Bytes::code(DEFAULT_LOCALS, DEFAULT_STACK, codes)
    }

    pub fn add(&mut self, data: &[u8]) {
        for i in 0 .. data.len() { self.data[self.index + i] = data[i] }
        self.index += data.len();
    }

    pub fn constant(mut n: usize) -> Bytes {
        let mut b = Bytes { data: [0; 128], index: 0 };
        for _i in 0 .. mem::size_of::<usize>() {
            b.data[b.index] = (n & 0xff) as u8;
            b.index += 1;
            n = n >> 8;
        }
        b
    }

    pub fn data(data: &[u8]) -> Bytes {
        let mut b = Bytes { data: [0; 128], index: 0 };
        b.add(data);
        b
    }

    // pub fn constant_sint(n: isize) -> Bytes {
    //     let mut b = Bytes { data: [0; 128], index: 0 };
    //     let mut raw: usize = if n >= 0 { (n as usize) << 1 } else { ((n as usize) << 1) ^ ((0 - 1) as usize) };
    //     while raw > 128 {
    //         b.data[b.index] = ((raw & 0x7f) as u8) | 0x80;
    //         b.index += 1;
    //         raw >>= 7;
    //     }
    //     b.data[b.index] = raw as u8;
    //     b.index += 1;
    //     b
    // }

    pub fn to_bytes(&self) -> &[u8] {
        &self.data[0 .. self.index]
    }
}


const HEAP_SIZE: usize = 512;
const CONSTANT_POOL_SIZE: usize = 256;

pub struct Platform {
    heap_data: [u8; HEAP_SIZE],
    constant_data: [u8; CONSTANT_POOL_SIZE],
    constant_index: usize,
    pub constant_offsets: [u32; 16],
    constant_offsets_index: usize,
}

impl Platform {
    pub fn new() -> Platform {
        Platform {
            heap_data: [0; HEAP_SIZE],
            constant_data: [0; CONSTANT_POOL_SIZE],
            constant_index: 0,
            constant_offsets: [0u32; 16],
            constant_offsets_index: 0,
        }
    }

    pub fn with(constants: &[Bytes]) -> Platform {
        let mut p = Platform::new();
        for c in constants { p.add_constant(c.to_bytes()) }
        p
    }

    pub fn add_constant(&mut self, data: &[u8]) {
        // align:
        let bits = mem::size_of::<usize>() - 1;
        self.constant_index = (self.constant_index + bits) & !bits;
        self.constant_offsets[self.constant_offsets_index] = self.constant_index as u32;
        self.constant_offsets_index += 1;
        for i in 0 .. data.len() { self.constant_data[self.constant_index + i] = data[i] }
        self.constant_index += data.len();
    }

    pub fn get_constant(&self, index: usize) -> u32 {
        self.constant_offsets[index] >> 2
    }

    pub fn to_runtime(&mut self) -> Result<Runtime, RuntimeError> {
        let pool = &self.constant_data[0 .. self.constant_index];
        Runtime::new(pool, &mut self.heap_data, DEFAULT_GLOBALS, None)
    }

    pub fn to_timed_runtime(&mut self, current_time: Option<fn() -> usize>) -> Result<Runtime, RuntimeError> {
        let pool = &self.constant_data[0 .. self.constant_index];
        Runtime::new(pool, &mut self.heap_data, DEFAULT_GLOBALS, current_time)
    }

    pub fn execute0(&mut self, code_index: u32, args: &[usize]) -> Result<(), RuntimeError> {
        let mut results: [usize; 16] = [ 0; 16 ];
        self.to_runtime().and_then(|mut r| r.execute(code_index, args, &mut results, None, None)).map(|count| {
            assert_eq!(count, 0);
            ()
        })
    }

    pub fn execute1(&mut self, code_index: u32, args: &[usize]) -> Result<usize, RuntimeError> {
        let mut results: [usize; 16] = [ 0; 16 ];
        self.to_runtime().and_then(|mut r| r.execute(code_index, args, &mut results, None, None)).map(|count| {
            assert_eq!(count, 1);
            results[0]
        })
    }

    pub fn execute2(&mut self, code_index: u32, args: &[usize]) -> Result<(usize, usize), RuntimeError> {
        let mut results: [usize; 16] = [ 0; 16 ];
        self.to_runtime().and_then(|mut r| r.execute(code_index, args, &mut results, None, None)).map(|count| {
            assert_eq!(count, 2);
            (results[0], results[1])
        })
    }
}
