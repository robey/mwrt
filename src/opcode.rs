use core::mem;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Opcode {
    Break = 0x00,
    Nop = 0x01,
    Dup = 0x02,                         // (probably only hand-crafted code/tests)
    Return = 0x03,                      // return S1 items from stack
    New = 0x04,                         // S1(slots) S2(fill) -> obj S1
    Size = 0x05,                        // #slots(S1) -> S1
    LoadSlot = 0x08,                    // load slot #B from obj A -> A
    Immediate = 0x10,                   // N1 -> S1
    Constant = 0x11,                    // addr(constant N1) -> S1,
    LoadSlotN = 0x12,                   // S1[N1] -> S1
    StoreSlotN = 0x13,                  // S1[N1] := S2
    LoadLocalN = 0x14,                  // @N1 -> S1
    StoreLocalN = 0x15,                 // S1 -> @N1
    ReturnN = 0x1f,
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
