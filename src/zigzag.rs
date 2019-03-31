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

pub fn decode_int(bytes: &[u8], mut index: usize) -> Option<DecodedInt> {
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

pub fn decode_zigzag(bytes: &[u8], mut index: usize) -> Option<DecodedInt> {
    decode_int(bytes, index).map(|d| DecodedInt::new((d.value >> 1) ^ -(d.value & 1), d.new_index))
}


#[cfg(test)]
mod tests {
    use super::{decode_int, decode_zigzag, DecodedInt};

    #[test]
    fn int() {
        assert_eq!(decode_int(&[ 0 ], 0), Some(DecodedInt::new(0, 1)));
        assert_eq!(decode_int(&[ 1 ], 0), Some(DecodedInt::new(1, 1)));
        assert_eq!(decode_int(&[ 2 ], 0), Some(DecodedInt::new(2, 1)));
        assert_eq!(decode_int(&[ 0x7e ], 0), Some(DecodedInt::new(126, 1)));
        assert_eq!(decode_int(&[ 0x7f ], 0), Some(DecodedInt::new(127, 1)));
        assert_eq!(decode_int(&[ 0x80, 0x01 ], 0), Some(DecodedInt::new(128, 2)));
        assert_eq!(decode_int(&[ 0x82, 0x40 ], 0), Some(DecodedInt::new(8194, 2)));
        assert_eq!(decode_int(&[ 0x80, 0x01, 0x80 ], 1), Some(DecodedInt::new(1, 2)));
        assert_eq!(decode_int(&[ 0x80, 0x80, 0x80, 0x80, 0x02 ], 0), Some(DecodedInt::new(0x20000000, 5)));

        assert_eq!(decode_int(&[ 0x80 ], 0), None);
        assert_eq!(decode_int(&[ 0 ], 1), None);
        assert_eq!(decode_int(&[ 0 ], 3), None);
        assert_eq!(decode_int(&[ 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 1 ], 0), None);
    }

    #[test]
    fn zigzag() {
        assert_eq!(decode_zigzag(&[ 0 ], 0), Some(DecodedInt::new(0, 1)));
        assert_eq!(decode_zigzag(&[ 1 ], 0), Some(DecodedInt::new(-1, 1)));
        assert_eq!(decode_zigzag(&[ 2 ], 0), Some(DecodedInt::new(1, 1)));
        assert_eq!(decode_zigzag(&[ 0x7e ], 0), Some(DecodedInt::new(63, 1)));
        assert_eq!(decode_zigzag(&[ 0x7f ], 0), Some(DecodedInt::new(-64, 1)));
        assert_eq!(decode_zigzag(&[ 0x80, 0x01 ], 0), Some(DecodedInt::new(64, 2)));
        assert_eq!(decode_zigzag(&[ 0x82, 0x40 ], 0), Some(DecodedInt::new(4097, 2)));
        assert_eq!(decode_zigzag(&[ 0x80, 0x01, 0x80 ], 1), Some(DecodedInt::new(-1, 2)));
        assert_eq!(decode_zigzag(&[ 0x80, 0x80, 0x80, 0x80, 0x02 ], 0), Some(DecodedInt::new(0x10000000, 5)));

        assert_eq!(decode_zigzag(&[ 0x80 ], 0), None);
        assert_eq!(decode_zigzag(&[ 0 ], 1), None);
        assert_eq!(decode_zigzag(&[ 0 ], 3), None);
        assert_eq!(decode_zigzag(&[ 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 1 ], 0), None);
    }
}
