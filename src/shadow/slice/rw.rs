use super::macros::{
    impl_read_primitive, impl_read_primitives, impl_slice_common, impl_slice_ro, impl_slice_wo,
    impl_write_primitive, impl_write_primitives,
};

/// Read-write slice wrapper.
///
/// Provides full read and write access to a byte slice with bounds-checked
/// methods for reading/writing primitives and copying data.
#[derive(Debug)]
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
