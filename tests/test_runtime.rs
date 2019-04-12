mod helpers;

use core::mem;
use mwrt::{Binary, Opcode, Unary};
use helpers::{Bytes, Platform};

const BINARY_ADD: &[u8] = &[ Opcode::Binary as u8, (Binary::Add as u8) << 1 ];
const BINARY_SUB: &[u8] = &[ Opcode::Binary as u8, (Binary::Subtract as u8) << 1 ];
const BINARY_MUL: &[u8] = &[ Opcode::Binary as u8, (Binary::Multiply as u8) << 1 ];
const BINARY_DIV: &[u8] = &[ Opcode::Binary as u8, (Binary::Divide as u8) << 1 ];
const BINARY_MOD: &[u8] = &[ Opcode::Binary as u8, (Binary::Modulo as u8) << 1 ];
const BINARY_EQ: &[u8] = &[ Opcode::Binary as u8, (Binary::Equals as u8) << 1 ];
const BINARY_LT: &[u8] = &[ Opcode::Binary as u8, (Binary::LessThan as u8) << 1 ];
const BINARY_LE: &[u8] = &[ Opcode::Binary as u8, (Binary::LessOrEqual as u8) << 1 ];
const BINARY_OR: &[u8] = &[ Opcode::Binary as u8, (Binary::BitOr as u8) << 1 ];
const BINARY_AND: &[u8] = &[ Opcode::Binary as u8, (Binary::BitAnd as u8) << 1 ];
const BINARY_XOR: &[u8] = &[ Opcode::Binary as u8, (Binary::BitXor as u8) << 1 ];
const BINARY_LSL: &[u8] = &[ Opcode::Binary as u8, (Binary::ShiftLeft as u8) << 1 ];
const BINARY_LSR: &[u8] = &[ Opcode::Binary as u8, (Binary::ShiftRight as u8) << 1 ];
const BINARY_ASR: &[u8] = &[ Opcode::Binary as u8, (Binary::SignShiftRight as u8) << 1 ];
const BREAK: &[u8] = &[ Opcode::Break as u8 ];
const DUP: &[u8] = &[ Opcode::Dup as u8 ];
const LOAD_GLOBAL_0: &[u8] = &[ Opcode::LoadGlobalN as u8, 0 ];
const LOAD_GLOBAL_1: &[u8] = &[ Opcode::LoadGlobalN as u8, 2 ];
const LOAD_LOCAL_0: &[u8] = &[ Opcode::LoadLocalN as u8, 0 ];
const LOAD_LOCAL_1: &[u8] = &[ Opcode::LoadLocalN as u8, 2 ];
const NEW: &[u8] = &[ Opcode::New as u8 ];
const NEW_3_2: &[u8] = &[ Opcode::NewNN as u8, 6, 4 ];
const NOP: &[u8] = &[ Opcode::Nop as u8 ];
const PUSH_N30: &[u8] = &[ Opcode::Immediate as u8, 59 ];
const PUSH_N1: &[u8] = &[ Opcode::Immediate as u8, 1 ];
const PUSH_0: &[u8] = &[ Opcode::Immediate as u8, 0 ];
const PUSH_1: &[u8] = &[ Opcode::Immediate as u8, 2 ];
const PUSH_2: &[u8] = &[ Opcode::Immediate as u8, 4 ];
const PUSH_30: &[u8] = &[ Opcode::Immediate as u8, 60 ];
const PUSH_64: &[u8] = &[ Opcode::Immediate as u8, 0x80, 1 ];
const PUSH_128: &[u8] = &[ Opcode::Immediate as u8, 0x80, 2 ];
const PUSH_CONST_1: &[u8] = &[ Opcode::Constant as u8, 2 ];
const RETURN: &[u8] = &[ Opcode::Return as u8 ];
const RETURN_1: &[u8] = &[ Opcode::ReturnN as u8, 2 ];
const SIZE: &[u8] = &[ Opcode::Size as u8 ];
const SLOT_0: &[u8] = &[ Opcode::LoadSlotN as u8, 0 ];
const SLOT_1: &[u8] = &[ Opcode::LoadSlotN as u8, 2 ];
const SLOT_2: &[u8] = &[ Opcode::LoadSlotN as u8, 4 ];
const STORE_LOCAL_0: &[u8] = &[ Opcode::StoreLocalN as u8, 0 ];
const STORE_LOCAL_1: &[u8] = &[ Opcode::StoreLocalN as u8, 2 ];
const STORE_LOCAL_10: &[u8] = &[ Opcode::StoreLocalN as u8, 20 ];
const STORE_GLOBAL_0: &[u8] = &[ Opcode::StoreGlobalN as u8, 0 ];
const STORE_GLOBAL_1: &[u8] = &[ Opcode::StoreGlobalN as u8, 2 ];
const STORE_GLOBAL_10: &[u8] = &[ Opcode::StoreGlobalN as u8, 20 ];
const STORE_SLOT_0: &[u8] = &[ Opcode::StoreSlotN as u8, 0 ];
// const STORE_SLOT_1: &[u8] = &[ Opcode::StoreSlotN as u8, 2 ];
const STORE_SLOT_2: &[u8] = &[ Opcode::StoreSlotN as u8, 4 ];
const UNARY_NOT: &[u8] = &[ Opcode::Unary as u8, (Unary::Not as u8) << 1 ];
const UNARY_NEG: &[u8] = &[ Opcode::Unary as u8, (Unary::Negative as u8) << 1 ];
const UNARY_BITNOT: &[u8] = &[ Opcode::Unary as u8, (Unary::BitNot as u8) << 1 ];


#[test]
fn out_of_memory() {
    let mut p = Platform::with(&[ Bytes::code(63, 63, &[ NOP ]) ]);
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
    let mut p = Platform::with(&[ Bytes::basic_code(&[ BREAK ]) ]);
    assert_eq!(format!("{:?}", p.execute0(0, &[])), "Err(Break at [frame code=0 pc=0 sp=0])");
}

#[test]
fn skip_nop() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ NOP, BREAK ]) ]);
    assert_eq!(format!("{:?}", p.execute0(0, &[])), "Err(Break at [frame code=0 pc=1 sp=0])");
}

#[test]
fn immediate_and_return() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_1, RETURN ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(128));
}

#[test]
fn immediate_dup_and_return() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, DUP, PUSH_2, RETURN ]) ]);
    assert_eq!(p.execute2(0, &[]).ok(), Some((128, 128)));
}

#[test]
fn constant_and_return() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_CONST_1, SLOT_0, PUSH_1, RETURN ]), Bytes::constant(300) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(300));
}

#[test]
fn new_object_and_load_slot() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_2, NEW_3_2, SLOT_0, PUSH_1, RETURN ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(128));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_2, NEW_3_2, SLOT_1, PUSH_1, RETURN ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(2));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_2, NEW_3_2, SLOT_2, PUSH_1, RETURN ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(0));
}

#[test]
fn new_object_errors() {
    // 128 is too big
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_0, NEW, SLOT_0, PUSH_1, RETURN ]) ]);
    assert_eq!(format!("{:?}", p.execute1(0, &[])), "Err(InvalidSize at [frame code=0 pc=5 sp=0])");

    // more slots to fill than are allocated
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_1, PUSH_2, NEW, SLOT_0, PUSH_1, RETURN ]) ]);
    assert_eq!(format!("{:?}", p.execute1(0, &[])), "Err(OutOfBounds at [frame code=0 pc=4 sp=0])");

    // we made a heap that can't actually hold a 64-slot object and also any stack frame at all
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_64, PUSH_0, NEW, SLOT_0, PUSH_1, RETURN ]) ]);
    assert_eq!(format!("{:?}", p.execute1(0, &[])), "Err(OutOfMemory at [frame code=0 pc=5 sp=0])");

    // there aren't enough fields on the stack
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_64, PUSH_2, PUSH_2, NEW, SLOT_0, PUSH_1, RETURN ]) ]);
    assert_eq!(format!("{:?}", p.execute1(0, &[])), "Err(StackUnderflow at [frame code=0 pc=7 sp=1])");
}

#[test]
fn new_object_and_store_slot() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[
        PUSH_128, PUSH_2, NEW_3_2, DUP, PUSH_1, STORE_SLOT_0, SLOT_0, PUSH_1, RETURN
    ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(1));

    let mut p = Platform::with(&[ Bytes::basic_code(&[
        PUSH_128, PUSH_2, NEW_3_2, DUP, PUSH_1, STORE_SLOT_0, SLOT_1, PUSH_1, RETURN
    ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(2));

    let mut p = Platform::with(&[ Bytes::basic_code(&[
        PUSH_128, PUSH_2, NEW_3_2, DUP, PUSH_1, STORE_SLOT_2, SLOT_2, PUSH_1, RETURN
    ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(1));

    let mut p = Platform::with(&[ Bytes::basic_code(&[
        PUSH_128, PUSH_2, NEW_3_2, DUP, PUSH_1, STORE_SLOT_2, SLOT_1, PUSH_1, RETURN
    ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(2));
}

#[test]
fn constant_object_and_load_slot() {
    let mut p = Platform::with(&[
        Bytes::basic_code(&[ PUSH_CONST_1, SLOT_0, PUSH_1, RETURN ]),
        Bytes::data(&[ 5, 0, 0, 0, 0, 0, 0, 0 ]),
    ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(5));

    let mut p = Platform::with(&[
        Bytes::basic_code(&[ PUSH_CONST_1, SLOT_2, PUSH_1, RETURN ]),
        Bytes::data(&[ 5, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 1, 1, 1, 1, 6, 0, 0, 0, 0, 0, 0, 0 ]),
    ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(6));
}

#[test]
fn object_size() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_2, NEW_3_2, SIZE, PUSH_1, RETURN ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(4));

    let mut p = Platform::with(&[
        Bytes::basic_code(&[ PUSH_CONST_1, SIZE, PUSH_1, RETURN ]),
        Bytes::data(&[ 0, 0, 0, 0, 1, 0, 0, 0 ]),
    ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(8 / mem::size_of::<usize>()));
}

#[test]
fn load_and_store_local() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, STORE_LOCAL_0, PUSH_2, LOAD_LOCAL_0, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(128));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, STORE_LOCAL_0, PUSH_2, STORE_LOCAL_1, LOAD_LOCAL_0, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(128));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, STORE_LOCAL_0, PUSH_2, STORE_LOCAL_1, LOAD_LOCAL_1, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(2));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, STORE_LOCAL_10 ]) ]);
    assert_eq!(format!("{:?}", p.execute1(0, &[])), "Err(OutOfBounds at [frame code=0 pc=3 sp=1])");
}

#[test]
fn load_and_store_global() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, STORE_GLOBAL_0, PUSH_2, LOAD_GLOBAL_0, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(128));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, STORE_GLOBAL_0, PUSH_2, STORE_GLOBAL_1, LOAD_GLOBAL_0, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(128));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, STORE_GLOBAL_0, PUSH_2, STORE_GLOBAL_1, LOAD_GLOBAL_1, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(2));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, STORE_GLOBAL_10 ]) ]);
    assert_eq!(format!("{:?}", p.execute1(0, &[])), "Err(OutOfBounds at [frame code=0 pc=3 sp=1])");
}

#[test]
fn unary() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, UNARY_NOT, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(0));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, UNARY_NOT, UNARY_NOT, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(1));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, UNARY_NEG, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some((-128 as isize) as usize));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_0, UNARY_BITNOT, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some((-1 as isize) as usize));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_1, UNARY_BITNOT, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some((-2 as isize) as usize));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_1, &[ Opcode::Unary as u8, 50 ], RETURN_1 ]) ]);
    assert_eq!(format!("{:?}", p.execute1(0, &[])), "Err(UnknownOpcode at [frame code=0 pc=2 sp=0])");
}

#[test]
fn binary_math() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_ADD, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(158));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_N30, BINARY_ADD, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(98));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_SUB, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(98));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_N30, BINARY_SUB, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(158));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_N30, PUSH_128, BINARY_SUB, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some((-158 as isize) as usize));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_MUL, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(3840));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_N30, BINARY_MUL, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some((-3840 as isize) as usize));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_DIV, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(4));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_MOD, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(8));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_1, PUSH_1, &[ Opcode::Binary as u8, 50 ], RETURN_1 ]) ]);
    assert_eq!(format!("{:?}", p.execute1(0, &[])), "Err(UnknownOpcode at [frame code=0 pc=4 sp=0])");
}

#[test]
fn binary_compare() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_EQ, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(0));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_30, PUSH_30, BINARY_EQ, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(1));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_LT, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(0));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_LT, UNARY_NOT, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(1));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_30, PUSH_128, BINARY_LT, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(1));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_N30, PUSH_30, BINARY_LT, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(1));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_30, PUSH_30, BINARY_LE, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(1));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_30, PUSH_128, BINARY_LE, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(1));
}

#[test]
fn binary_bit() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_OR, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(158));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_N1, PUSH_30, BINARY_OR, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some((-1 as isize) as usize));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_AND, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(0));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_N1, PUSH_30, BINARY_AND, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(30));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_128, PUSH_30, BINARY_XOR, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(158));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_N1, PUSH_30, BINARY_XOR, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some((-31 as isize) as usize));
}

#[test]
fn binary_shift() {
    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_30, PUSH_2, BINARY_LSL, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(120));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_30, PUSH_2, BINARY_LSR, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(7));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_N30, PUSH_2, BINARY_ASR, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some((-8 as isize) as usize));

    let mut p = Platform::with(&[ Bytes::basic_code(&[ PUSH_30, PUSH_2, BINARY_ASR, RETURN_1 ]) ]);
    assert_eq!(p.execute1(0, &[]).ok(), Some(7));
}

// FIXME: error cases
