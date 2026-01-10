/// Generates read method(s) for a single primitive type.
macro_rules! impl_read_primitive {
    // Single byte types - no endianness suffix
    (u8) => {
        /// Reads a `u8` at the given offset.
        ///
        /// # Panics
        /// Panics if `offset >= len()`.
        #[inline]
        pub fn read_u8_at(&self, offset: usize) -> u8 {
            self.0[offset]
        }
    };
    (i8) => {
        /// Reads an `i8` at the given offset.
        ///
        /// # Panics
        /// Panics if `offset >= len()`.
        #[inline]
        pub fn read_i8_at(&self, offset: usize) -> i8 {
            self.0[offset] as i8
        }
    };
    // Multi-byte types - le/be variants
    ($type:ty, $size:literal) => {
        paste::paste! {
            #[doc = "Reads a little-endian `" $type "` at the given offset."]
            #[doc = ""]
            #[doc = "# Panics"]
            #[doc = "Panics if `offset + " $size " > len()`."]
            #[inline]
            pub fn [<read_ $type _le_at>](&self, offset: usize) -> $type {
                assert!(
                    offset + $size <= self.0.len(),
                    "read out of bounds: offset {} + size {} > len {}",
                    offset, $size, self.0.len()
                );
                <$type>::from_le_bytes(self.0[offset..offset + $size].try_into().unwrap())
            }

            #[doc = "Reads a big-endian `" $type "` at the given offset."]
            #[doc = ""]
            #[doc = "# Panics"]
            #[doc = "Panics if `offset + " $size " > len()`."]
            #[inline]
            pub fn [<read_ $type _be_at>](&self, offset: usize) -> $type {
                assert!(
                    offset + $size <= self.0.len(),
                    "read out of bounds: offset {} + size {} > len {}",
                    offset, $size, self.0.len()
                );
                <$type>::from_be_bytes(self.0[offset..offset + $size].try_into().unwrap())
            }
        }
    };
}

/// Generates read methods for all standard primitive types.
macro_rules! impl_read_primitives {
    () => {
        impl_read_primitive!(u8);
        impl_read_primitive!(i8);
        impl_read_primitive!(u16, 2);
        impl_read_primitive!(i16, 2);
        impl_read_primitive!(u32, 4);
        impl_read_primitive!(i32, 4);
    };
}

/// Generates write method(s) for a single primitive type.
macro_rules! impl_write_primitive {
    // Single byte types - no endianness suffix
    (u8) => {
        /// Writes a `u8` at the given offset.
        ///
        /// # Panics
        /// Panics if `offset >= len()`.
        #[inline]
        pub fn write_u8_at(&mut self, offset: usize, value: u8) {
            self.0[offset] = value;
        }
    };
    (i8) => {
        /// Writes an `i8` at the given offset.
        ///
        /// # Panics
        /// Panics if `offset >= len()`.
        #[inline]
        pub fn write_i8_at(&mut self, offset: usize, value: i8) {
            self.0[offset] = value as u8;
        }
    };
    // Multi-byte types - le/be variants
    ($type:ty, $size:literal) => {
        paste::paste! {
            #[doc = "Writes a little-endian `" $type "` at the given offset."]
            #[doc = ""]
            #[doc = "# Panics"]
            #[doc = "Panics if `offset + " $size " > len()`."]
            #[inline]
            pub fn [<write_ $type _le_at>](&mut self, offset: usize, value: $type) {
                assert!(
                    offset + $size <= self.0.len(),
                    "write out of bounds: offset {} + size {} > len {}",
                    offset, $size, self.0.len()
                );
                self.0[offset..offset + $size].copy_from_slice(&value.to_le_bytes());
            }

            #[doc = "Writes a big-endian `" $type "` at the given offset."]
            #[doc = ""]
            #[doc = "# Panics"]
            #[doc = "Panics if `offset + " $size " > len()`."]
            #[inline]
            pub fn [<write_ $type _be_at>](&mut self, offset: usize, value: $type) {
                assert!(
                    offset + $size <= self.0.len(),
                    "write out of bounds: offset {} + size {} > len {}",
                    offset, $size, self.0.len()
                );
                self.0[offset..offset + $size].copy_from_slice(&value.to_be_bytes());
            }
        }
    };
}

/// Generates write methods for all standard primitive types.
macro_rules! impl_write_primitives {
    () => {
        impl_write_primitive!(u8);
        impl_write_primitive!(i8);
        impl_write_primitive!(u16, 2);
        impl_write_primitive!(i16, 2);
        impl_write_primitive!(u32, 4);
        impl_write_primitive!(i32, 4);
    };
}

/// Generates common slice methods (len, is_empty).
macro_rules! impl_slice_common {
    () => {
        /// Returns the length of the slice.
        #[inline]
        pub fn len(&self) -> usize {
            self.0.len()
        }

        /// Returns true if the slice is empty.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.0.is_empty()
        }
    };
}

/// Generates read-only slice methods (copy_to_slice, copy_to_slice_at, read primitives).
macro_rules! impl_slice_ro {
    () => {
        /// Copies the entire slice to the destination buffer.
        ///
        /// # Panics
        /// Panics if destination length doesn't match slice length.
        #[inline]
        pub fn copy_to_slice(&self, dest: &mut [u8]) {
            dest.copy_from_slice(self.0);
        }

        /// Copies data starting at `offset` to the destination buffer.
        ///
        /// # Panics
        /// Panics if the range exceeds slice bounds.
        #[inline]
        pub fn copy_to_slice_at(&self, offset: usize, dest: &mut [u8]) {
            dest.copy_from_slice(&self.0[offset..offset + dest.len()]);
        }

        impl_read_primitives!();
    };
}

/// Generates write-only slice methods (copy_from_slice, copy_from_slice_at, fill, fill_at, write primitives).
macro_rules! impl_slice_wo {
    () => {
        /// Copies the source buffer to the entire slice.
        ///
        /// # Panics
        /// Panics if source length doesn't match slice length.
        #[inline]
        pub fn copy_from_slice(&mut self, src: &[u8]) {
            self.0.copy_from_slice(src);
        }

        /// Copies the source buffer starting at `offset`.
        ///
        /// # Panics
        /// Panics if the range exceeds slice bounds.
        #[inline]
        pub fn copy_from_slice_at(&mut self, offset: usize, src: &[u8]) {
            self.0[offset..offset + src.len()].copy_from_slice(src);
        }

        /// Fills the entire slice with the given value.
        #[inline]
        pub fn fill(&mut self, value: u8) {
            self.0.fill(value);
        }

        /// Fills `len` bytes starting at `offset` with the given value.
        ///
        /// # Panics
        /// Panics if the range exceeds slice bounds.
        #[inline]
        pub fn fill_at(&mut self, offset: usize, len: usize, value: u8) {
            self.0[offset..offset + len].fill(value);
        }

        impl_write_primitives!();
    };
}

pub(super) use impl_read_primitive;
pub(super) use impl_read_primitives;
pub(super) use impl_slice_common;
pub(super) use impl_slice_ro;
pub(super) use impl_slice_wo;
pub(super) use impl_write_primitive;
pub(super) use impl_write_primitives;
