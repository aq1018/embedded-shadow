use super::macros::{
    impl_read_primitive, impl_read_primitives, impl_slice_common, impl_slice_ro, impl_slice_wo,
    impl_write_primitive, impl_write_primitives,
};

/// Read-write slice wrapper.
///
/// Provides full read and write access to a byte slice with bounds-checked
/// methods for reading/writing primitives and copying data.
pub struct RWSlice<'a>(&'a mut [u8]);

impl<'a> RWSlice<'a> {
    /// Creates a new read-write slice wrapper.
    #[inline]
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self(slice)
    }

    impl_slice_common!();
    impl_slice_ro!();
    impl_slice_wo!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_roundtrip() {
        let mut data = [0u8; 4];
        let mut slice = RWSlice::new(&mut data);

        slice.write_u32_le_at(0, 0x12345678);
        assert_eq!(slice.read_u32_le_at(0), 0x12345678);
    }

    #[test]
    fn read_modify_write() {
        let mut data = [0x00, 0x00, 0x00, 0x01];
        let mut slice = RWSlice::new(&mut data);

        let value = slice.read_u32_le_at(0);
        slice.write_u32_le_at(0, value | 0x80000000);

        assert_eq!(data, [0x00, 0x00, 0x00, 0x81]);
    }

    #[test]
    #[should_panic]
    fn out_of_bounds() {
        let mut data = [0u8; 4];
        RWSlice::new(&mut data).read_u32_le_at(1);
    }
}
