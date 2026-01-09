use core::cell::UnsafeCell;

use bitmaps::{Bits, BitsImpl};

use crate::{
    persist::PersistTrigger,
    policy::AddressPolicy,
    shadow::{HostShadow, KernelShadow},
    table::ShadowTable,
    types::StagingBuffer,
};

pub struct NoStage;

pub struct WithStage<SB: StagingBuffer> {
    pub(crate) sb: SB,
}

pub struct ShadowStorageBase<const TS: usize, const BS: usize, const BC: usize, AP, PT, SS>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    BitsImpl<BC>: Bits,
{
    pub(crate) table: UnsafeCell<ShadowTable<TS, BS, BC>>,
    pub(crate) policy: AP,
    pub(crate) persist: PT,
    pub(crate) stage: UnsafeCell<SS>,
}

pub type ShadowStorage<const TS: usize, const BS: usize, const BC: usize, AP, PT> =
    ShadowStorageBase<TS, BS, BC, AP, PT, NoStage>;

impl<const TS: usize, const BS: usize, const BC: usize, AP, PT>
    ShadowStorageBase<TS, BS, BC, AP, PT, NoStage>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    BitsImpl<BC>: Bits,
{
    pub fn new(policy: AP, persist: PT) -> Self {
        Self {
            table: UnsafeCell::new(ShadowTable::new()),
            policy,
            persist,
            stage: UnsafeCell::new(NoStage),
        }
    }

    /// Upgrade this storage to staged mode by supplying a staging implementation.
    pub fn with_staging<SB: StagingBuffer>(
        self,
        sb: SB,
    ) -> ShadowStorageBase<TS, BS, BC, AP, PT, WithStage<SB>> {
        ShadowStorageBase {
            table: self.table,
            policy: self.policy,
            persist: self.persist,
            stage: UnsafeCell::new(WithStage { sb }),
        }
    }
}

impl<const TS: usize, const BS: usize, const BC: usize, AP, PT, SS>
    ShadowStorageBase<TS, BS, BC, AP, PT, SS>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    BitsImpl<BC>: Bits,
{
    pub fn host_shadow(&self) -> HostShadow<'_, TS, BS, BC, AP, PT, SS> {
        HostShadow::new(self)
    }

    pub fn kernel_shadow(&self) -> KernelShadow<'_, TS, BS, BC, AP, PT, SS> {
        KernelShadow::new(self)
    }
}
