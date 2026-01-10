use super::macros::{
    impl_slice_common, impl_slice_wo, impl_write_primitive, impl_write_primitives,
};

/// Write-only slice wrapper.
///
/// Provides write-only access to a byte slice with bounds-checked
/// methods for writing primitives and copying data in. The underlying
/// data cannot be read through this wrapper.
pub struct WOSlice<'a>(&'a mut [u8]);

impl<'a> WOSlice<'a> {
    /// Creates a new write-only slice wrapper.
    #[inline]
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self(slice)
    }

    impl_slice_common!();
    impl_slice_wo!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_from_slice() {
        let mut data = [0u8; 4];

        // Full copy
        WOSlice::new(&mut data).copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(data, [0xAA, 0xBB, 0xCC, 0xDD]);

        // Partial copy at offset
        data = [0u8; 4];
        WOSlice::new(&mut data).copy_from_slice_at(1, &[0x11, 0x22]);
        assert_eq!(data, [0x00, 0x11, 0x22, 0x00]);
    }

    #[test]
    fn fill() {
        let mut data = [0u8; 4];

        // Full fill
        WOSlice::new(&mut data).fill(0xFF);
        assert_eq!(data, [0xFF, 0xFF, 0xFF, 0xFF]);

        // Partial fill at offset
        data = [0u8; 4];
        WOSlice::new(&mut data).fill_at(1, 2, 0xAA);
        assert_eq!(data, [0x00, 0xAA, 0xAA, 0x00]);
    }

    #[test]
    fn write_primitives() {
        let mut data = [0u8; 4];

        WOSlice::new(&mut data).write_u32_le_at(0, 0x12345678);
        assert_eq!(data, [0x78, 0x56, 0x34, 0x12]);

        WOSlice::new(&mut data).write_u32_be_at(0, 0x12345678);
        assert_eq!(data, [0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    #[should_panic]
    fn copy_from_slice_length_mismatch() {
        let mut data = [0u8; 4];
        WOSlice::new(&mut data).copy_from_slice(&[1, 2, 3]);
    }

    #[test]
    #[should_panic]
    fn copy_from_slice_at_out_of_bounds() {
        let mut data = [0u8; 4];
        WOSlice::new(&mut data).copy_from_slice_at(3, &[1, 2]);
    }

    #[test]
    #[should_panic]
    fn fill_at_out_of_bounds() {
        let mut data = [0u8; 4];
        WOSlice::new(&mut data).fill_at(3, 2, 0xFF);
    }

    #[test]
    #[should_panic]
    fn write_out_of_bounds() {
        let mut data = [0u8; 4];
        WOSlice::new(&mut data).write_u32_le_at(1, 0);
    }
}
