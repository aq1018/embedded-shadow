//! Utility functions for address and block calculations.
//!
//! These helpers are useful when implementing custom [`AccessPolicy`](crate::shadow::AccessPolicy)
//! or [`PersistPolicy`](crate::shadow::PersistPolicy) types that need to reason about
//! address ranges and block boundaries.

use crate::shadow::ShadowError;

/// Calculates which blocks are spanned by an address range.
///
/// Returns the inclusive range `(start_block, end_block)` for the given
/// address and length, or an error if the range is invalid.
///
/// # Arguments
/// * `addr` - Starting address
/// * `len` - Length of the range in bytes
///
/// # Type Parameters
/// * `TS` - Total size of the shadow table
/// * `BS` - Block size in bytes
/// * `BC` - Block count
///
/// # Errors
/// * [`ShadowError::ZeroLength`] - if `len` is 0
/// * [`ShadowError::OutOfBounds`] - if the range exceeds table bounds
///
/// # Example
/// ```
/// use embedded_shadow::shadow::helpers::block_span;
///
/// // 64-byte table, 16-byte blocks, 4 blocks
/// // Address 20 with length 10 spans bytes 20-29, which is block 1
/// let (start, end) = block_span::<64, 16, 4>(20, 10).unwrap();
/// assert_eq!((start, end), (1, 1));
///
/// // Address 14 with length 4 spans bytes 14-17, crossing blocks 0 and 1
/// let (start, end) = block_span::<64, 16, 4>(14, 4).unwrap();
/// assert_eq!((start, end), (0, 1));
/// ```
pub fn block_span<const TS: usize, const BS: usize, const BC: usize>(
    addr: u16,
    len: usize,
) -> Result<(usize, usize), ShadowError> {
    let (offset, end) = range_span::<TS>(addr, len)?;
    let sb = offset / BS;
    let eb = (end - 1) / BS; // inclusive

    if eb >= BC {
        return Err(ShadowError::OutOfBounds);
    }

    Ok((sb, eb))
}

/// Calculates the byte offset range for an address and length.
///
/// Returns `(start_offset, end_offset)` where end is exclusive,
/// or an error if the range is invalid.
///
/// # Arguments
/// * `addr` - Starting address
/// * `len` - Length of the range in bytes
///
/// # Type Parameters
/// * `TS` - Total size of the shadow table
///
/// # Errors
/// * [`ShadowError::ZeroLength`] - if `len` is 0
/// * [`ShadowError::OutOfBounds`] - if the range exceeds table bounds
pub fn range_span<const TS: usize>(addr: u16, len: usize) -> Result<(usize, usize), ShadowError> {
    if len == 0 {
        return Err(ShadowError::ZeroLength);
    }

    let offset = addr as usize;
    let end = offset.checked_add(len).ok_or(ShadowError::OutOfBounds)?;

    if end > TS {
        return Err(ShadowError::OutOfBounds);
    }

    Ok((offset, end))
}

#[test]
fn block_span_edge_cases() {
    // Zero length
    assert_eq!(block_span::<16, 4, 4>(0, 0), Err(ShadowError::ZeroLength));

    // Out of bounds
    assert_eq!(block_span::<16, 4, 4>(15, 2), Err(ShadowError::OutOfBounds));

    // Single byte at block boundary
    assert_eq!(block_span::<16, 4, 4>(4, 1), Ok((1, 1)));

    // Exact block
    assert_eq!(block_span::<16, 4, 4>(4, 4), Ok((1, 1)));

    // Spanning all blocks
    assert_eq!(block_span::<16, 4, 4>(0, 16), Ok((0, 3)));

    // Last byte of table
    assert_eq!(block_span::<16, 4, 4>(15, 1), Ok((3, 3)));
}
