use heapless::Vec;

use crate::{AddressPolicy, ShadowError, staged::internal::Staged, table::ShadowTable};

#[derive(Clone, Copy)]
struct StagedWrite {
    addr: u16,
    len: u16,
    off: u16, // offset into data vec
}

pub(crate) struct PatchStaging<const DATA_CAP: usize, const ENTRY_CAP: usize> {
    data: Vec<u8, DATA_CAP>,
    entries: Vec<StagedWrite, ENTRY_CAP>,
}

impl<const DATA_CAP: usize, const ENTRY_CAP: usize> PatchStaging<DATA_CAP, ENTRY_CAP> {
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            entries: Vec::new(),
        }
    }

    #[inline]
    fn assert_nonzero(len: usize) -> Result<(), ShadowError> {
        if len == 0 {
            return Err(ShadowError::ZeroLength);
        }
        Ok(())
    }

    #[inline]
    fn range_span<const T: usize>(addr: u16, len: usize) -> Result<(usize, usize), ShadowError> {
        Self::assert_nonzero(len)?;
        let off = addr as usize;
        let end = off.checked_add(len).ok_or(ShadowError::OutOfBounds)?;
        if end > T {
            return Err(ShadowError::OutOfBounds);
        }
        Ok((off, end))
    }

    #[inline]
    fn push_bytes(&mut self, bytes: &[u8]) -> Result<u16, ShadowError> {
        let off = self.data.len();
        if off + bytes.len() > DATA_CAP {
            return Err(ShadowError::StageFull);
        }
        // heapless::Vec::extend_from_slice returns Result<(), _>
        self.data
            .extend_from_slice(bytes)
            .map_err(|_| ShadowError::StageFull)?;
        Ok(off as u16)
    }
}

impl<const T: usize, const B: usize, const W: usize, const DATA_CAP: usize, const ENTRY_CAP: usize>
    Staged<T, B, W> for PatchStaging<DATA_CAP, ENTRY_CAP>
{
    fn has_staged(&self) -> bool {
        !self.entries.is_empty()
    }

    fn write_range_staged(
        &mut self,
        _table: &ShadowTable<T, B, W>,
        addr: u16,
        data: &[u8],
    ) -> Result<(), ShadowError> {
        let _ = Self::range_span::<T>(addr, data.len())?;

        // Optimization: overwrite-in-place if exact same span already staged.
        if let Some(i) = self
            .entries
            .iter()
            .position(|e| e.addr == addr && e.len as usize == data.len())
        {
            let e = self.entries[i];
            let start = e.off as usize;
            let end = start + data.len();
            self.data[start..end].copy_from_slice(data);
            return Ok(());
        }

        if self.entries.is_full() {
            return Err(ShadowError::StageFull);
        }

        let off = self.push_bytes(data)?;

        self.entries
            .push(StagedWrite {
                addr,
                len: data.len() as u16,
                off,
            })
            .map_err(|_| ShadowError::StageFull)?;

        Ok(())
    }

    fn read_range_overlay(
        &self,
        table: &ShadowTable<T, B, W>,
        addr: u16,
        out: &mut [u8],
    ) -> Result<(), ShadowError> {
        let (req_off, req_end) = Self::range_span::<T>(addr, out.len())?;

        // Start with committed bytes
        table.read_range(addr, out)?;

        // Overlay patches in order (later entries overwrite earlier => "last wins")
        for e in self.entries.iter() {
            let e_start = e.addr as usize;
            let e_end = e_start + e.len as usize;

            let ov_start = core::cmp::max(req_off, e_start);
            let ov_end = core::cmp::min(req_end, e_end);
            if ov_start >= ov_end {
                continue;
            }

            let n = ov_end - ov_start;
            let out_i = ov_start - req_off;
            let data_i = (e.off as usize) + (ov_start - e_start);

            out[out_i..out_i + n].copy_from_slice(&self.data[data_i..data_i + n]);
        }

        Ok(())
    }

    fn action<P: AddressPolicy>(
        &mut self,
        table: &mut ShadowTable<T, B, W>,
        policy: &P,
    ) -> Result<bool, ShadowError> {
        let mut needs_persist = false;

        // Pre-validate all ranges to avoid partial apply if you want stronger semantics.
        for e in self.entries.iter() {
            let _ = Self::range_span::<T>(e.addr, e.len as usize)?;
        }

        // Apply in insertion order (last wins for overlaps).
        for e in self.entries.iter() {
            let start = e.off as usize;
            let end = start + e.len as usize;
            let slice = &self.data[start..end];

            table.write_range(e.addr, slice)?;
            table.mark_dirty(e.addr, e.len as usize)?;

            if policy.triggers_persist(e.addr, e.len as usize) {
                needs_persist = true;
            }
        }

        self.data.clear();
        self.entries.clear();

        Ok(needs_persist)
    }
}
