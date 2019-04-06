mod helpers;

use mwrt::{Opcode, Runtime};
use helpers::{Bytes, Platform};

const NOP: &[u8] = &[ Opcode::Nop as u8 ];
const PUSH_128: &[u8] = &[ Opcode::Immediate as u8, 0x80, 2 ];
const PUSH_1: &[u8] = &[ Opcode::Immediate as u8, 2 ];
const RETURN: &[u8] = &[ Opcode::Return as u8 ];


#[test]
fn out_of_memory() {
    let mut p = Platform::with(&[ Bytes::code(63, 63, &[ &[ Opcode::Nop as u8 ] ]) ]);
    assert_eq!(format!("{:?}", p.execute0(0, &[])), "Err(OutOfMemory)");
}

#[test]
fn unknown() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ &[ 0xff ] ]) ]);
    assert_eq!(format!("{:?}", p.execute0(0, &[])), "Err(UnknownOpcode at [frame code=0 pc=0 sp=0])");
}

#[test]
fn incomplete_immediate() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ &[ Opcode::Immediate as u8, 0x80 ] ]) ]);
    assert_eq!(format!("{:?}", p.execute0(0, &[])), "Err(TruncatedCode at [frame code=0 pc=0 sp=0])");
}

#[test]
fn debugger_break() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ &[ Opcode::Break as u8 ] ]) ]);
    assert_eq!(format!("{:?}", p.execute0(0, &[])), "Err(Break at [frame code=0 pc=0 sp=0])");
}

#[test]
fn skip_nop() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ &[ Opcode::Nop as u8, Opcode::Break as u8 ] ]) ]);
    assert_eq!(format!("{:?}", p.execute0(0, &[])), "Err(Break at [frame code=0 pc=1 sp=0])");
}

#[test]
fn immediate_and_return() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_1, RETURN ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(128))
}
