use super::macros::{
    impl_slice_common, impl_slice_wo, impl_write_primitive, impl_write_primitives,
};

/// Write-only slice wrapper.
///
/// Provides write-only access to a byte slice with bounds-checked
/// methods for writing primitives and copying data in. The underlying
/// data cannot be read through this wrapper.
#[derive(Debug)]
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
    fn wo_slice_operations() {
        let mut data = [0u8; 4];

        // Test copy operations
        WOSlice::new(&mut data).copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(data, [0xAA, 0xBB, 0xCC, 0xDD]);

        data = [0u8; 4];
        WOSlice::new(&mut data).copy_from_slice_at(1, &[0x11, 0x22]);
        assert_eq!(data, [0x00, 0x11, 0x22, 0x00]);

        // Test fill operations
        WOSlice::new(&mut data).fill(0xFF);
        assert_eq!(data, [0xFF, 0xFF, 0xFF, 0xFF]);

        data = [0u8; 4];
        WOSlice::new(&mut data).fill_at(1, 2, 0xAA);
        assert_eq!(data, [0x00, 0xAA, 0xAA, 0x00]);

        // Test primitive writes
        WOSlice::new(&mut data).write_u32_le_at(0, 0x12345678);
        assert_eq!(data, [0x78, 0x56, 0x34, 0x12]);

        WOSlice::new(&mut data).write_u32_be_at(0, 0x12345678);
        assert_eq!(data, [0x12, 0x34, 0x56, 0x78]);
    }
}
