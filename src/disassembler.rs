use core::fmt;
use crate::decode_int::decode_sint;
use crate::opcode::{FIRST_N1_OPCODE, FIRST_N2_OPCODE, LAST_N_OPCODE, Opcode};

pub struct Instruction {
    offset: usize,
    opcode: Opcode,
    n1: isize,
    n2: isize,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04x}: ", self.offset)?;
        match self.opcode {
            Opcode::Break => write!(f, "BREAK"),
            Opcode::Nop => write!(f, "NOP"),
            Opcode::Dup => write!(f, "DUP"),
            Opcode::Return => write!(f, "RET"),
            Opcode::LoadSlot => write!(f, "LD [*]"),
            Opcode::Immediate => write!(f, "LD #{}", self.n1),
            Opcode::Constant => write!(f, "LD %{}", self.n1),
            Opcode::LoadSlotN => write!(f, "LD [#{}]", self.n1),
            Opcode::StoreSlotN => write!(f, "ST [#{}]", self.n1),
            Opcode::NewNN => write!(f, "NEW #{}, #{}", self.n1, self.n2),
            _ => write!(f, "???({:x})", self.opcode as u8),
        }
    }
}

impl Instruction {
}


pub struct Disassembler<'a> {
    bytecode: &'a [u8],
    index: usize,
}

impl<'a> Iterator for Disassembler<'a> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Instruction> {
        if self.index >= self.bytecode.len() { return None }
        decode_next(self.bytecode, self.index).map(|(instruction, new_index)| {
            self.index = new_index;
            instruction
        })
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

pub fn decode_next(bytes: &[u8], index: usize) -> Option<(Instruction, usize)> {
    if index >= bytes.len() { return None }

    let mut i = index;
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
                    return None;
                }
            }
        } else {
            return None;
        }
    }

    let instruction = Instruction { opcode: Opcode::from_u8(instruction), n1, n2, offset: index };
    Some((instruction, i))
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
    }
}
