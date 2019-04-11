mod helpers;

use core::mem;
use mwrt::Opcode;
use helpers::{Bytes, Platform};

const BREAK: &[u8] = &[ Opcode::Break as u8 ];
const DUP: &[u8] = &[ Opcode::Dup as u8 ];
const NEW: &[u8] = &[ Opcode::New as u8 ];
const NEW_3_2: &[u8] = &[ Opcode::NewNN as u8, 6, 4 ];
const NOP: &[u8] = &[ Opcode::Nop as u8 ];
const PUSH_0: &[u8] = &[ Opcode::Immediate as u8, 0 ];
const PUSH_1: &[u8] = &[ Opcode::Immediate as u8, 2 ];
const PUSH_2: &[u8] = &[ Opcode::Immediate as u8, 4 ];
const PUSH_64: &[u8] = &[ Opcode::Immediate as u8, 0x80, 1 ];
const PUSH_128: &[u8] = &[ Opcode::Immediate as u8, 0x80, 2 ];
const PUSH_CONST_1: &[u8] = &[ Opcode::Constant as u8, 2 ];
const RETURN: &[u8] = &[ Opcode::Return as u8 ];
const SIZE: &[u8] = &[ Opcode::Size as u8 ];
const SLOT_0: &[u8] = &[ Opcode::LoadSlotN as u8, 0 ];
const SLOT_1: &[u8] = &[ Opcode::LoadSlotN as u8, 2 ];
const SLOT_2: &[u8] = &[ Opcode::LoadSlotN as u8, 4 ];
const STORE_SLOT_0: &[u8] = &[ Opcode::StoreSlotN as u8, 0 ];
// const STORE_SLOT_1: &[u8] = &[ Opcode::StoreSlotN as u8, 2 ];
const STORE_SLOT_2: &[u8] = &[ Opcode::StoreSlotN as u8, 4 ];


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
    assert_eq!(format!("{:?}", p.execute1(0, &[])), "Err(InvalidSize at [frame code=0 pc=4 sp=0])");

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


// FIXME: error cases
