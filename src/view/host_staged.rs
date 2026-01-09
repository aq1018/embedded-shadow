use crate::{AddressPolicy, HostView, PersistTrigger, types::StagingBuffer};
use bitmaps::{Bits, BitsImpl};

pub struct HostViewStaged<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT, SB>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    BitsImpl<BC>: Bits,
    SB: StagingBuffer,
{
    base: HostView<'a, TS, BS, BC, AP, PT>,
    sb: &'a mut SB,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT, SB>
    HostViewStaged<'a, TS, BS, BC, AP, PT, SB>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    BitsImpl<BC>: Bits,
    SB: StagingBuffer,
{
    pub(crate) fn new(base: HostView<'a, TS, BS, BC, AP, PT>, sb: &'a mut SB) -> Self {
        Self { base, sb }
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT, SB>
    HostViewStaged<'a, TS, BS, BC, AP, PT, SB>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    BitsImpl<BC>: Bits,
    SB: StagingBuffer,
{
    pub fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), crate::ShadowError> {
        self.base.read_range(addr, out)
    }

    pub fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), crate::ShadowError> {
        self.base.write_range(addr, data)
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT, SB>
    HostViewStaged<'a, TS, BS, BC, AP, PT, SB>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    BitsImpl<BC>: Bits,
    SB: StagingBuffer,
{
    pub fn read_range_overlay(&self, addr: u16, out: &mut [u8]) -> Result<(), crate::ShadowError> {
        if !self.base.policy.can_read(addr, out.len()) {
            return Err(crate::ShadowError::Denied);
        }

        self.base.read_range(addr, out)?;
        self.sb.apply_overlay(addr, out)?;
        Ok(())
    }

    pub fn write_range_staged(&mut self, addr: u16, data: &[u8]) -> Result<(), crate::ShadowError> {
        if !self.base.policy.can_write(addr, data.len()) {
            return Err(crate::ShadowError::Denied);
        }

        self.sb.write_staged(addr, data)
    }

    pub fn action(&mut self) -> Result<(), crate::ShadowError> {
        if !self.sb.any_staged() {
            return Ok(());
        }

        self.sb
            .for_each_staged(|addr, data| self.base.write_range(addr, data))?;

        self.sb.clear_staged()
    }
}
