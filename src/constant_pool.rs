use core::fmt;
use crate::decode_int::decode_uint;

pub struct ConstantPool<'rom> {
    data: &'rom [u8],
}

impl<'rom> ConstantPool<'rom> {
    pub fn new(data: &'rom [u8]) -> ConstantPool {
        ConstantPool { data }
    }

    pub fn get(&self, mut index: usize) -> Option<&'rom [u8]> {
        let mut i = 0;
        while index > 0 {
            match self.next(i) {
                None => { return None },
                Some(n) => i = n,
            }
            index -= 1;
        }

        decode_uint(self.data, i).map(|len| { &self.data[len.new_index .. len.new_index + len.value as usize] })
    }

    fn next(&self, offset: usize) -> Option<usize> {
        decode_uint(self.data, offset).map(|size| size.new_index + (size.value as usize))
    }
}

impl<'rom, 'heap> fmt::Debug for ConstantPool<'rom> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", "ConstantPool(")?;
        let mut i = 0;
        let mut item = self.get(i);
        while let Some(x) = item {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{} = {:?}", i, x)?;
            i += 1;
            item = self.get(i);
        }
        write!(f, ")")
    }
}


#[cfg(test)]
mod tests {
    use super::ConstantPool;

    #[test]
    fn get() {
        let pool = ConstantPool::new(&[ 0x02, 0xff, 0xfe, 0x01, 0x23, 0x03, 1, 2, 3 ]);
        assert_eq!(pool.get(0), Some(&[ 0xff, 0xfe ][..]));
        assert_eq!(pool.get(1), Some(&[ 0x23 ][..]));
        assert_eq!(pool.get(2), Some(&[ 1, 2, 3 ][..]));
        assert_eq!(pool.get(3), None);
    }
}
