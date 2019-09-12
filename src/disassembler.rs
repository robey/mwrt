use core::fmt;
use crate::decode_int::decode_sint;
use crate::error::ErrorCode;
use crate::opcode::{Binary, FIRST_N1_OPCODE, FIRST_N2_OPCODE, LAST_N_OPCODE, Opcode, Unary};

pub struct Instruction {
    pub offset: usize,
    pub opcode: Opcode,
    pub n1: isize,
    pub n2: isize,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04x}: ", self.offset)?;
        match self.opcode {
            Opcode::Break => write!(f, "BREAK"),
            Opcode::Nop => write!(f, "NOP"),
            Opcode::Dup => write!(f, "DUP"),
            Opcode::Drop => write!(f, "DROP"),
            Opcode::Call => write!(f, "CALL"),
            Opcode::Return => write!(f, "RET"),
            Opcode::New => write!(f, "NEW"),
            Opcode::Size => write!(f, "SIZE"),
            Opcode::LoadSlot => write!(f, "LD [*]"),
            Opcode::StoreSlot => write!(f, "ST [*]"),
            Opcode::If => write!(f, "IF"),
            Opcode::Immediate => write!(f, "LD #{}", self.n1),
            Opcode::Constant => write!(f, "LD %{}", self.n1),
            Opcode::LoadSlotN => write!(f, "LD [#{}]", self.n1),
            Opcode::StoreSlotN => write!(f, "ST [#{}]", self.n1),
            Opcode::LoadLocalN => write!(f, "LD @{}", self.n1),
            Opcode::StoreLocalN => write!(f, "ST @{}", self.n1),
            Opcode::LoadGlobalN => write!(f, "LD ${}", self.n1),
            Opcode::StoreGlobalN => write!(f, "ST ${}", self.n1),
            Opcode::Unary => match Unary::from_usize(self.n1 as usize) {
                Unary::Not => write!(f, "NOT"),
                Unary::Negative => write!(f, "NEG"),
                Unary::BitNot => write!(f, "INV"),
                _ => write!(f, "?unary?"),
            },
            Opcode::Binary => match Binary::from_usize(self.n1 as usize) {
                Binary::Add => write!(f, "ADD"),
                Binary::Subtract => write!(f, "SUB"),
                Binary::Multiply => write!(f, "MUL"),
                Binary::Divide => write!(f, "DIV"),
                Binary::Modulo => write!(f, "MOD"),
                Binary::Equals => write!(f, "EQ"),
                Binary::LessThan => write!(f, "LT"),
                Binary::LessOrEqual => write!(f, "LE"),
                Binary::BitOr => write!(f, "OR"),
                Binary::BitAnd => write!(f, "AND"),
                Binary::BitXor => write!(f, "XOR"),
                Binary::ShiftLeft => write!(f, "LSL"),
                Binary::ShiftRight => write!(f, "LSR"),
                Binary::SignShiftRight => write!(f, "ASR"),
                _ => write!(f, "?binary?"),
            },
            Opcode::CallN => write!(f, "CALL #{}", self.n1),
            Opcode::ReturnN => write!(f, "RET #{}", self.n1),
            Opcode::Jump => write!(f, "JUMP {:04x}", self.n1),
            Opcode::NewNN => write!(f, "NEW #{}, #{}", self.n1, self.n2),
            _ => write!(f, "???({:x})", self.opcode as u8),
        }
    }
}


pub struct Disassembler<'a> {
    bytecode: &'a [u8],
    index: u16,
}

impl<'a> Iterator for Disassembler<'a> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Instruction> {
        if (self.index as usize) >= self.bytecode.len() { return None }
        match decode_next(self.bytecode, self.index).map(|(instruction, new_index)| {
            self.index = new_index;
            instruction
        }) {
            Ok(instruction) => Some(instruction),
            Err(_) => None
        }
    }
}

pub fn disassemble<'a>(bytes: &'a [u8]) -> Disassembler<'a> {
    Disassembler { bytecode: bytes, index: 0 }
}

pub fn disassemble_to_string<W: fmt::Write>(bytes: &[u8], f: &mut W) -> fmt::Result {
    for i in disassemble(bytes) {
        write!(f, "{}\n", i)?;
    }
    Ok(())
}

pub fn decode_next(bytes: &[u8], index: u16) -> Result<(Instruction, u16), ErrorCode> {
    let mut i = index as usize;
    if i >= bytes.len() { return Err(ErrorCode::TruncatedCode) }
    let instruction = bytes[i];
    i += 1;

    // immediates?
    let mut n1: isize = 0;
    let mut n2: isize = 0;
    if instruction >= FIRST_N1_OPCODE && instruction < LAST_N_OPCODE {
        if let Some(d1) = decode_sint(bytes, i) {
            n1 = d1.value;
            i = d1.new_index;
            if instruction >= FIRST_N2_OPCODE {
                if let Some(d2) = decode_sint(bytes, i) {
                    n2 = d2.value;
                    i = d2.new_index;
                } else {
                    return Err(ErrorCode::TruncatedCode);
                }
            }
        } else {
            return Err(ErrorCode::TruncatedCode);
        }
    }

    let instruction = Instruction { opcode: Opcode::from_u8(instruction), n1, n2, offset: index as usize };
    Ok((instruction, i as u16))
}


#[cfg(test)]
mod tests {
    use mwgc::StringBuffer;
    use crate::opcode::Opcode;
    use super::disassemble_to_string;

    #[test]
    fn disassemble() {
        let bytes: &[u8] = &[
            Opcode::Break as u8, Opcode::Nop as u8, Opcode::Return as u8, Opcode::LoadSlot as u8,
            Opcode::Immediate as u8, 2, Opcode::Constant as u8, 0x80, 2,
            Opcode::LoadSlotN as u8, 0x82, 4,
        ];
        let mut buffer: [u8; 256] = [0; 256];
        let mut b = StringBuffer::new(&mut buffer);
        disassemble_to_string(&bytes, &mut b).ok();
        assert_eq!(
            b.to_str(),
            "0000: BREAK\n0001: NOP\n0002: RET\n0003: LD [*]\n0004: LD #1\n0006: LD %128\n0009: LD [#257]\n"
        );

        let bytes: &[u8] = &[
            Opcode::Drop as u8, Opcode::StoreSlot as u8, Opcode::If as u8,
            Opcode::Jump as u8, 0xfe, 7
        ];
        let mut buffer: [u8; 256] = [0; 256];
        let mut b = StringBuffer::new(&mut buffer);
        disassemble_to_string(&bytes, &mut b).ok();
        assert_eq!(
            b.to_str(),
            "0000: DROP\n0001: ST [*]\n0002: IF\n0003: JUMP 01ff\n"
        );

        let bytes: &[u8] = &[
            Opcode::Dup as u8, Opcode::New as u8, Opcode::Size as u8, Opcode::StoreSlotN as u8, 0x84, 4,
            Opcode::LoadLocalN as u8, 0x80, 0x80, 1, Opcode::StoreLocalN as u8, 6,
            Opcode::ReturnN as u8, 2,
        ];
        let mut buffer: [u8; 256] = [0; 256];
        let mut b = StringBuffer::new(&mut buffer);
        disassemble_to_string(&bytes, &mut b).ok();
        assert_eq!(
            b.to_str(),
            "0000: DUP\n0001: NEW\n0002: SIZE\n0003: ST [#258]\n0006: LD @8192\n000a: ST @3\n000c: RET #1\n"
        );

        let bytes: &[u8] = &[
            Opcode::NewNN as u8, 0x80, 0x80, 0x80, 1, 0x82, 0x80, 0x80, 1,
            Opcode::LoadGlobalN as u8, 12, Opcode::StoreGlobalN as u8, 100, Opcode::Call as u8,
            Opcode::CallN as u8, 0x80, 1,
        ];
        let mut buffer: [u8; 256] = [0; 256];
        let mut b = StringBuffer::new(&mut buffer);
        disassemble_to_string(&bytes, &mut b).ok();
        assert_eq!(
            b.to_str(),
            "0000: NEW #1048576, #1048577\n0009: LD $6\n000b: ST $50\n000d: CALL\n000e: CALL #64\n"
        );

        let bytes: &[u8] = &[
            Opcode::Unary as u8, 0,
            Opcode::Unary as u8, 2,
            Opcode::Unary as u8, 4,
        ];
        let mut buffer: [u8; 256] = [0; 256];
        let mut b = StringBuffer::new(&mut buffer);
        disassemble_to_string(&bytes, &mut b).ok();
        assert_eq!(
            b.to_str(),
            "0000: NOT\n0002: NEG\n0004: INV\n"
        );

        let bytes: &[u8] = &[
            Opcode::Binary as u8, 0,
            Opcode::Binary as u8, 2,
            Opcode::Binary as u8, 4,
            Opcode::Binary as u8, 6,
            Opcode::Binary as u8, 8,
            Opcode::Binary as u8, 10,
            Opcode::Binary as u8, 12,
            Opcode::Binary as u8, 14,
            Opcode::Binary as u8, 16,
        ];
        let mut buffer: [u8; 256] = [0; 256];
        let mut b = StringBuffer::new(&mut buffer);
        disassemble_to_string(&bytes, &mut b).ok();
        assert_eq!(
            b.to_str(),
            "0000: ADD\n0002: SUB\n0004: MUL\n0006: DIV\n0008: MOD\n000a: EQ\n000c: LT\n000e: LE\n0010: OR\n"
        );

        let bytes: &[u8] = &[
            Opcode::Binary as u8, 18,
            Opcode::Binary as u8, 20,
            Opcode::Binary as u8, 22,
            Opcode::Binary as u8, 24,
            Opcode::Binary as u8, 26,
        ];
        let mut buffer: [u8; 256] = [0; 256];
        let mut b = StringBuffer::new(&mut buffer);
        disassemble_to_string(&bytes, &mut b).ok();
        assert_eq!(
            b.to_str(),
            "0000: AND\n0002: XOR\n0004: LSL\n0006: LSR\n0008: ASR\n"
        );
    }
}
