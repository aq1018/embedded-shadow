use crate::ShadowError;

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
    assert_eq!(
        block_span::<256, 16, 16>(0, 0),
        Err(ShadowError::ZeroLength)
    );

    // Out of bounds
    assert_eq!(
        block_span::<256, 16, 16>(15, 2),
        Err(ShadowError::OutOfBounds)
    );

    // Single byte at block boundary
    assert_eq!(block_span::<256, 16, 16>(4, 1), Ok((1, 1)));

    // Exact block
    assert_eq!(block_span::<256, 16, 16>(4, 4), Ok((1, 1)));

    // Spanning all blocks
    assert_eq!(block_span::<256, 16, 16>(0, 16), Ok((0, 3)));

    // Last byte of table
    assert_eq!(block_span::<256, 16, 16>(15, 1), Ok((3, 3)));
}
