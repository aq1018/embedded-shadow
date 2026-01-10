use super::macros::{
    impl_read_primitive, impl_read_primitives, impl_slice_common, impl_slice_ro,
    impl_try_read_primitive, impl_try_read_primitives,
};

/// Read-only slice wrapper.
///
/// Provides read-only access to a byte slice with bounds-checked
/// methods for reading primitives and copying data out.
#[derive(Debug)]
pub struct ROSlice<'a>(&'a [u8]);

impl<'a> ROSlice<'a> {
    /// Creates a new read-only slice wrapper.
    #[inline]
    pub fn new(slice: &'a [u8]) -> Self {
        Self(slice)
    }

    impl_slice_common!();
    impl_slice_ro!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ro_slice_operations() {
        let data = [0x78, 0x56, 0x34, 0x12];
        let slice = ROSlice::new(&data);

        // Test copy operations
        let mut dest = [0u8; 4];
        slice.copy_to_slice(&mut dest);
        assert_eq!(dest, data);

        let mut dest = [0u8; 2];
        slice.copy_to_slice_at(1, &mut dest);
        assert_eq!(dest, [0x56, 0x34]);

        // Test primitive reads
        assert_eq!(slice.read_u32_le_at(0), 0x12345678);
        assert_eq!(slice.read_u32_be_at(0), 0x78563412);
        assert_eq!(slice.read_u8_at(0), 0x78);
    }

    #[test]
    #[should_panic(expected = "read out of bounds")]
    fn ro_slice_read_u32_out_of_bounds() {
        let data = [0u8; 4];
        let slice = ROSlice::new(&data);
        slice.read_u32_le_at(1); // offset 1 + size 4 > len 4
    }

    #[test]
    #[should_panic(expected = "read out of bounds")]
    fn ro_slice_read_u16_out_of_bounds() {
        let data = [0u8; 2];
        let slice = ROSlice::new(&data);
        slice.read_u16_le_at(1); // offset 1 + size 2 > len 2
    }

    #[test]
    fn try_read_operations() {
        let data = [0x78, 0x56, 0x34, 0x12];
        let slice = ROSlice::new(&data);

        // Successful reads
        assert_eq!(slice.try_read_u8_at(0), Some(0x78));
        assert_eq!(slice.try_read_i8_at(0), Some(0x78));
        assert_eq!(slice.try_read_u16_le_at(0), Some(0x5678));
        assert_eq!(slice.try_read_u16_be_at(0), Some(0x7856));
        assert_eq!(slice.try_read_u32_le_at(0), Some(0x12345678));
        assert_eq!(slice.try_read_u32_be_at(0), Some(0x78563412));

        // Out of bounds returns None
        assert_eq!(slice.try_read_u8_at(4), None);
        assert_eq!(slice.try_read_u16_le_at(3), None);
        assert_eq!(slice.try_read_u32_le_at(1), None);
    }

    #[test]
    fn try_copy_to_slice_at_operations() {
        let data = [0x78, 0x56, 0x34, 0x12];
        let slice = ROSlice::new(&data);

        // Successful copy
        let mut dest = [0u8; 2];
        assert_eq!(slice.try_copy_to_slice_at(1, &mut dest), Some(()));
        assert_eq!(dest, [0x56, 0x34]);

        // Out of bounds returns None
        let mut dest = [0u8; 4];
        assert_eq!(slice.try_copy_to_slice_at(1, &mut dest), None);
    }
}
