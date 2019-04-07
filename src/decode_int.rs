use core::mem;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecodedInt {
    pub value: isize,
    pub new_index: usize,
}

impl DecodedInt {
    fn new(value: isize, new_index: usize) -> DecodedInt {
        DecodedInt { value, new_index }
    }
}

const MAX_SHIFT: usize = mem::size_of::<usize>() * 8;

/// uint is encoded as "varint":
/// 7 bits at a time, LSB first, high bit is set on all but the last byte.
pub fn decode_uint(bytes: &[u8], mut index: usize) -> Option<DecodedInt> {
    if index >= bytes.len() { return None }
    let mut value: isize = 0;
    let mut shift: usize = 0;

    while bytes[index] & 0x80 != 0 {
        value = value | (((bytes[index] & 0x7f) as isize) << shift);
        index += 1;
        if index >= bytes.len() { return None }
        shift += 7;
        if shift >= MAX_SHIFT { return None }
    }
    value = value | ((bytes[index] as isize) << shift);
    Some(DecodedInt::new(value, index + 1))
}

/// sint is encoded as "zigzag":
/// number is shifted left one place, adding a "sign bit" as the lowest bit.
/// when the sign bit is set, the rest of the number is inverted, so -1 is
/// encoded as 0x01, -2 as 0x03, and so on. the result is then encoded the
/// same as a varint.
pub fn decode_sint(bytes: &[u8], index: usize) -> Option<DecodedInt> {
    decode_uint(bytes, index).map(|d| DecodedInt::new((d.value >> 1) ^ -(d.value & 1), d.new_index))
}

pub fn decode_unaligned(bytes: &[u8], index: usize) -> Option<DecodedInt> {
    let end = index + mem::size_of::<usize>();
    if end > bytes.len() { return None }
    let mut rv: usize = 0;
    let mut shift: usize = 0;
    for i in index .. end {
        rv |= (bytes[i] as usize) << shift;
        shift += 8;
    }
    Some(DecodedInt::new(rv as isize, end))
}


#[cfg(test)]
mod tests {
    use super::{decode_sint, decode_uint, decode_unaligned, DecodedInt};

    #[test]
    fn uint() {
        assert_eq!(decode_uint(&[ 0 ], 0), Some(DecodedInt::new(0, 1)));
        assert_eq!(decode_uint(&[ 1 ], 0), Some(DecodedInt::new(1, 1)));
        assert_eq!(decode_uint(&[ 2 ], 0), Some(DecodedInt::new(2, 1)));
        assert_eq!(decode_uint(&[ 0x7e ], 0), Some(DecodedInt::new(126, 1)));
        assert_eq!(decode_uint(&[ 0x7f ], 0), Some(DecodedInt::new(127, 1)));
        assert_eq!(decode_uint(&[ 0x80, 0x01 ], 0), Some(DecodedInt::new(128, 2)));
        assert_eq!(decode_uint(&[ 0x82, 0x40 ], 0), Some(DecodedInt::new(8194, 2)));
        assert_eq!(decode_uint(&[ 0x80, 0x01, 0x80 ], 1), Some(DecodedInt::new(1, 2)));
        assert_eq!(decode_uint(&[ 0x80, 0x80, 0x80, 0x80, 0x02 ], 0), Some(DecodedInt::new(0x20000000, 5)));

        assert_eq!(decode_uint(&[ 0x80 ], 0), None);
        assert_eq!(decode_uint(&[ 0 ], 1), None);
        assert_eq!(decode_uint(&[ 0 ], 3), None);
        assert_eq!(decode_uint(&[ 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 1 ], 0), None);
    }

    #[test]
    fn sint() {
        assert_eq!(decode_sint(&[ 0 ], 0), Some(DecodedInt::new(0, 1)));
        assert_eq!(decode_sint(&[ 1 ], 0), Some(DecodedInt::new(-1, 1)));
        assert_eq!(decode_sint(&[ 2 ], 0), Some(DecodedInt::new(1, 1)));
        assert_eq!(decode_sint(&[ 0x7e ], 0), Some(DecodedInt::new(63, 1)));
        assert_eq!(decode_sint(&[ 0x7f ], 0), Some(DecodedInt::new(-64, 1)));
        assert_eq!(decode_sint(&[ 0x80, 0x01 ], 0), Some(DecodedInt::new(64, 2)));
        assert_eq!(decode_sint(&[ 0x82, 0x40 ], 0), Some(DecodedInt::new(4097, 2)));
        assert_eq!(decode_sint(&[ 0x80, 0x01, 0x80 ], 1), Some(DecodedInt::new(-1, 2)));
        assert_eq!(decode_sint(&[ 0x80, 0x80, 0x80, 0x80, 0x02 ], 0), Some(DecodedInt::new(0x10000000, 5)));

        assert_eq!(decode_sint(&[ 0x80 ], 0), None);
        assert_eq!(decode_sint(&[ 0 ], 1), None);
        assert_eq!(decode_sint(&[ 0 ], 3), None);
        assert_eq!(decode_sint(&[ 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 1 ], 0), None);
    }

    #[test]
    fn unaligned() {
        assert_eq!(decode_unaligned(&[ 0 ], 0), None);
        assert_eq!(decode_unaligned(&[ 0, 0, 0, 0, 0, 0, 0, 0 ], 0), Some(DecodedInt::new(0, 8)));
        assert_eq!(decode_unaligned(&[ 1, 0, 0, 0, 0, 0, 0, 0 ], 0), Some(DecodedInt::new(1, 8)));
        assert_eq!(decode_unaligned(&[ 255, 255, 255, 255, 255, 255, 255, 255 ], 0), Some(DecodedInt::new(-1, 8)));
        assert_eq!(decode_unaligned(&[ 44, 1, 0, 0, 0, 0, 0, 0 ], 0), Some(DecodedInt::new(300, 8)));
        assert_eq!(decode_unaligned(&[ 9, 9, 44, 1, 0, 0, 0, 0, 0, 0, 9 ], 2), Some(DecodedInt::new(300, 10)));
    }
}
