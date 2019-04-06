// helpers to make a runtime

use mwrt::{ConstantPool, Runtime, RuntimeError};

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
        let mut b = Bytes { data: [0; 128], index: 2 };
        b.data[0] = local_count as u8;
        b.data[1] = stack_count as u8;
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

    pub fn to_bytes(&self) -> &[u8] {
        &self.data[0 .. self.index]
    }
}


pub struct Platform {
    heap_data: [u8; 256],
    constant_data: [u8; 256],
    constant_index: usize,
}

impl Platform {
    pub fn new() -> Platform {
        Platform { heap_data: [0; 256], constant_data: [0; 256], constant_index: 0 }
    }

    pub fn with(constants: &[Bytes]) -> Platform {
        let mut p = Platform::new();
        for c in constants { p.add_constant(c.to_bytes()) }
        p
    }

    pub fn add_constant(&mut self, data: &[u8]) {
        // will need to actually support varint encoding if this passes 127 bytes
        assert!(data.len() < 128);
        self.constant_data[self.constant_index] = data.len() as u8;
        self.constant_index += 1;
        for i in 0 .. data.len() { self.constant_data[self.constant_index + i] = data[i] }
        self.constant_index += data.len();
    }

    pub fn to_runtime(&mut self) -> Runtime {
        let pool = ConstantPool::new(&self.constant_data[0 .. self.constant_index]);
        Runtime::new(pool, &mut self.heap_data)
    }

    pub fn execute0(&mut self, code_index: usize, args: &[usize]) -> Result<(), RuntimeError> {
        let mut results: [usize; 16] = [ 0; 16 ];
        self.to_runtime().execute(code_index, args, &mut results).map(|count| {
            assert_eq!(count, 0);
            ()
        })
    }

    pub fn execute1(&mut self, code_index: usize, args: &[usize]) -> Result<usize, RuntimeError> {
        let mut results: [usize; 16] = [ 0; 16 ];
        self.to_runtime().execute(code_index, args, &mut results).map(|count| {
            assert_eq!(count, 1);
            results[0]
        })
    }
}
