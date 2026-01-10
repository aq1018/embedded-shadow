use super::macros::{impl_read_primitive, impl_read_primitives, impl_slice_common, impl_slice_ro};

/// Read-only slice wrapper.
///
/// Provides read-only access to a byte slice with bounds-checked
/// methods for reading primitives and copying data out.
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
    fn copy_to_slice() {
        let data = [0x11, 0x22, 0x33, 0x44];
        let slice = ROSlice::new(&data);

        // Full copy
        let mut dest = [0u8; 4];
        slice.copy_to_slice(&mut dest);
        assert_eq!(dest, data);

        // Partial copy at offset
        let mut dest = [0u8; 2];
        slice.copy_to_slice_at(1, &mut dest);
        assert_eq!(dest, [0x22, 0x33]);
    }

    #[test]
    fn read_primitives() {
        let data = [0x78, 0x56, 0x34, 0x12];
        let slice = ROSlice::new(&data);

        assert_eq!(slice.read_u32_le_at(0), 0x12345678);
        assert_eq!(slice.read_u32_be_at(0), 0x78563412);
    }

    #[test]
    #[should_panic]
    fn copy_to_slice_length_mismatch() {
        let data = [0u8; 4];
        let mut dest = [0u8; 3];
        ROSlice::new(&data).copy_to_slice(&mut dest);
    }

    #[test]
    #[should_panic]
    fn copy_to_slice_at_out_of_bounds() {
        let data = [0u8; 4];
        let mut dest = [0u8; 2];
        ROSlice::new(&data).copy_to_slice_at(3, &mut dest);
    }

    #[test]
    #[should_panic]
    fn read_out_of_bounds() {
        let data = [0u8; 4];
        ROSlice::new(&data).read_u32_le_at(1);
    }
}
