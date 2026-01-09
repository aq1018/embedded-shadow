use heapless::Vec;

use crate::{ShadowError, types::StagingBuffer};

#[derive(Clone, Copy)]
struct StagedWrite {
    addr: u16,
    len: u16,
    off: u16, // offset into data vec
}

pub struct PatchStagingBuffer<const DC: usize, const EC: usize> {
    data: Vec<u8, DC>,
    entries: Vec<StagedWrite, EC>,
}

impl<const DC: usize, const EC: usize> PatchStagingBuffer<DC, EC> {
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            entries: Vec::new(),
        }
    }

    fn push_bytes(&mut self, bytes: &[u8]) -> Result<u16, ShadowError> {
        let off = self.data.len();
        if off + bytes.len() > DC {
            return Err(ShadowError::StageFull);
        }

        self.data
            .extend_from_slice(bytes)
            .map_err(|_| ShadowError::StageFull)?;

        Ok(off as u16)
    }
}

impl<const DC: usize, const EC: usize> StagingBuffer for PatchStagingBuffer<DC, EC> {
    fn any_staged(&self) -> bool {
        !self.entries.is_empty()
    }

    fn for_each_staged<F>(&self, mut f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), ShadowError>,
    {
        for e in self.entries.iter() {
            let buf = &self.data[e.off as usize..(e.off + e.len) as usize];
            f(e.addr, buf)?;
        }
        Ok(())
    }

    fn write_staged(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError> {
        let off = self.push_bytes(data)?;

        let entry = StagedWrite {
            addr,
            len: data.len() as u16,
            off,
        };

        self.entries
            .push(entry)
            .map_err(|_| ShadowError::StageFull)?;

        Ok(())
    }

    fn apply_overlay(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError> {
        if !self.any_staged() {
            return Ok(());
        }

        // overlay staged writes onto out
        for e in self.entries.iter() {
            let start = e.addr as usize;
            let end = start + e.len as usize;
            let out_start = addr as usize;
            let out_end = out_start + out.len();

            // Check for overlap
            if end <= out_start || start >= out_end {
                continue; // No overlap
            }

            // Calculate overlapping range
            let overlap_start = start.max(out_start);
            let overlap_end = end.min(out_end);

            let data_i = (overlap_start - start) as usize + e.off as usize;
            let out_i = (overlap_start - out_start) as usize;
            let n = overlap_end - overlap_start;

            // Write staged data into the output buffer
            out[out_i..out_i + n].copy_from_slice(&self.data[data_i..data_i + n]);
        }

        Ok(())
    }

    fn clear_staged(&mut self) -> Result<(), ShadowError> {
        self.data.clear();
        self.entries.clear();
        Ok(())
    }
}
