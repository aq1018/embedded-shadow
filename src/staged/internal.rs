use crate::{AddressPolicy, ShadowError, table::ShadowTable};

pub(crate) trait StagedImpl<const T: usize, const B: usize, const W: usize> {
    fn has_staged(&self) -> bool;

    fn write_range_staged(
        &mut self,
        table: &ShadowTable<T, B, W>,
        addr: u16,
        data: &[u8],
    ) -> Result<(), ShadowError>;

    fn read_range_overlay(
        &self,
        table: &ShadowTable<T, B, W>,
        addr: u16,
        out: &mut [u8],
    ) -> Result<(), ShadowError>;

    fn action<P: AddressPolicy>(
        &mut self,
        table: &mut ShadowTable<T, B, W>,
        policy: &P,
    ) -> Result<bool, ShadowError>;
}

#[cfg(feature = "staged-mirror")]
pub(crate) type DefaultStaging<const T: usize, const B: usize, const W: usize> =
    MirrorStaging<T, B, W>;

#[cfg(feature = "staged-patch")]
pub(crate) type DefaultStaging<const T: usize, const B: usize, const W: usize> =
    PatchStaging<64, 8>;
