use core::{cell::UnsafeCell, marker::PhantomData};

use bitmaps::{Bits, BitsImpl};

use crate::{
    persist::PersistTrigger,
    policy::{AccessPolicy, PersistPolicy},
    shadow::{HostShadow, KernelShadow},
    table::ShadowTable,
    types::StagingBuffer,
};

pub struct NoStage;

pub struct WithStage<SB: StagingBuffer> {
    pub(crate) sb: SB,
}

pub struct ShadowStorageBase<const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SS>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    BitsImpl<BC>: Bits,
{
    pub(crate) table: UnsafeCell<ShadowTable<TS, BS, BC>>,
    pub(crate) access_policy: AP,
    pub(crate) persist_policy: PP,
    pub(crate) persist_trigger: UnsafeCell<PT>,
    pub(crate) stage_state: UnsafeCell<SS>,
    _phantom: PhantomData<PK>,
}

pub type ShadowStorage<const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK> =
    ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, NoStage>;

impl<const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK>
    ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, NoStage>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    BitsImpl<BC>: Bits,
{
    pub fn new(policy: AP, persist: PP, trigger: PT) -> Self {
        Self {
            table: UnsafeCell::new(ShadowTable::new()),
            access_policy: policy,
            persist_policy: persist,
            persist_trigger: UnsafeCell::new(trigger),
            stage_state: UnsafeCell::new(NoStage),
            _phantom: PhantomData,
        }
    }

    /// Upgrade this storage to staged mode by supplying a staging implementation.
    pub fn with_staging<SB: StagingBuffer>(
        self,
        sb: SB,
    ) -> ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, WithStage<SB>> {
        ShadowStorageBase {
            table: self.table,
            access_policy: self.access_policy,
            persist_policy: self.persist_policy,
            persist_trigger: self.persist_trigger,
            stage_state: UnsafeCell::new(WithStage { sb }),
            _phantom: PhantomData,
        }
    }
}

impl<const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SS>
    ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, SS>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    BitsImpl<BC>: Bits,
{
    pub fn host_shadow(&self) -> HostShadow<'_, TS, BS, BC, AP, PP, PT, PK, SS> {
        HostShadow::new(self)
    }

    pub fn kernel_shadow(&self) -> KernelShadow<'_, TS, BS, BC, AP, PP, PT, PK, SS> {
        KernelShadow::new(self)
    }
}
