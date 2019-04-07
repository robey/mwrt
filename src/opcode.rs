use core::mem;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Opcode {
    Break = 0x00,
    Nop = 0x01,
    Return = 0x02,
    LoadSlot = 0x08,                    // load slot #B from obj A -> A
    Immediate = 0x10,                   // N1 -> S1
    Constant = 0x11,                    // addr(constant N1) -> S1,
    LoadSlotN = 0x12,                   // S1[N1] -> S1
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
