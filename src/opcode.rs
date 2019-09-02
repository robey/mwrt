use core::mem;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Opcode {
    // 0 immediates:
    Break = 0x00,
    Nop = 0x01,
    Dup = 0x02,                         // (probably only hand-crafted code/tests)
    Drop = 0x03,                        // (probably only hand-crafted code/tests)
    Call = 0x04,                        // call S2 with S1 args preceding
    Return = 0x05,                      // return S1 items from stack
    New = 0x06,                         // S1(slots) S2(fill_from_stack) -> obj S1
    Size = 0x07,                        // #slots(S1) -> S1
    LoadSlot = 0x08,                    // S1[S2] -> S1
    StoreSlot = 0x09,                   // S1[S2] := S3
    If = 0x0a,

    // 1 immediate:
    Immediate = 0x10,                   // N1 -> S1
    Constant = 0x11,                    // addr(constant N1) -> S1,
    LoadSlotN = 0x12,                   // S1[N1] -> S1
    StoreSlotN = 0x13,                  // S1[N1] := S2
    LoadLocalN = 0x14,                  // @N1 -> S1
    StoreLocalN = 0x15,                 // S1 -> @N1
    LoadGlobalN = 0x16,                 // $N1 -> S1
    StoreGlobalN = 0x17,                // S1 -> $N1
    Unary = 0x18,
    Binary = 0x19,
    CallN = 0x1a,                       // call S1 with N1 args preceding
    ReturnN = 0x1b,                     // return N1 items from stack
    Jump = 0x1c,

    // 2 immediates:
    NewNN = 0x20,                       // N1(slots) N2(fill) -> obj S1

    Unknown = 0xff,
}

// opcodes 0x1X have one immediate; 0x2X have two
pub const FIRST_N1_OPCODE: u8 = 0x10;
pub const FIRST_N2_OPCODE: u8 = 0x20;
pub const LAST_N_OPCODE: u8 = 0x30;

impl Opcode {
    // why isn't this automatic or derivable?
    pub fn from_u8(n: u8) -> Opcode {
        unsafe { mem::transmute(n) }
    }
}


#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unary {
    Not = 0,
    Negative = 1,
    BitNot = 2,
    Unknown = 0xff,
}

impl Unary {
    pub fn from_usize(n: usize) -> Unary {
        unsafe { mem::transmute(n) }
    }
}


#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binary {
    Add = 0,
    Subtract = 1,
    Multiply = 2,
    Divide = 3,
    Modulo = 4,
    Equals = 5,
    LessThan = 6,
    LessOrEqual = 7,
    BitOr = 8,
    BitAnd = 9,
    BitXor = 10,
    ShiftLeft = 11,
    ShiftRight = 12,
    SignShiftRight = 13,
    Unknown = 0xff,
}

impl Binary {
    pub fn from_usize(n: usize) -> Binary {
        unsafe { mem::transmute(n) }
    }
}
