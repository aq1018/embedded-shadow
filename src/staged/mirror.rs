use crate::ShadowError;

pub(crate) struct MirrorStaging<const T: usize, const B: usize, const W: usize> {
    mirror: [u8; T],
    staged_words: [u32; W],
}

impl<const T: usize, const B: usize, const W: usize> MirrorStaging<T, B, W> {
    pub const fn new() -> Self {
        Self {
            mirror: [0; T],
            staged_words: [0; W],
        }
    }

    fn assert_nonzero(len: usize) -> Result<(), ShadowError> {
        if len == 0 {
            return Err(ShadowError::ZeroLength);
        }
        Ok(())
    }

    fn range_span(addr: u16, len: usize) -> Result<(usize, usize), ShadowError> {
        Self::assert_nonzero(len)?;
        let off = addr as usize;
        let end = off.checked_add(len).ok_or(ShadowError::OutOfBounds)?;
        if end > T {
            return Err(ShadowError::OutOfBounds);
        }
        Ok((off, end))
    }

    fn block_span(addr: u16, len: usize) -> Result<(usize, usize), ShadowError> {
        let (off, end) = Self::range_span(addr, len)?;
        let sb = off / B;
        let eb = (end - 1) / B;
        Ok((sb, eb))
    }

    fn bit_get(&self, block: usize) -> bool {
        let wi = block / 32;
        let bi = block % 32;
        if wi >= W {
            return false;
        }
        (self.staged_words[wi] & (1u32 << bi)) != 0
    }

    fn bit_set(&mut self, block: usize) {
        let wi = block / 32;
        let bi = block % 32;
        if wi < W {
            self.staged_words[wi] |= 1u32 << bi;
        }
    }

    fn bit_clear(&mut self, block: usize) {
        let wi = block / 32;
        let bi = block % 32;
        if wi < W {
            self.staged_words[wi] &= !(1u32 << bi);
        }
    }

    fn any_bits(&self) -> bool {
        self.staged_words.iter().any(|&w| w != 0)
    }
}
