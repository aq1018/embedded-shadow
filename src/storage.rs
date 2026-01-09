#![allow(unsafe_code)]

use core::{cell::UnsafeCell, marker::PhantomData};

use bitmaps::{Bits, BitsImpl};

use crate::{
    ShadowError,
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

/// Write function type for [`ShadowStorageBase::load_defaults`].
pub type WriteFn = dyn FnMut(u16, &[u8]) -> Result<(), ShadowError>;

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

    /// Load initial values into the shadow table without marking dirty.
    ///
    /// Use this during system initialization to populate the shadow
    /// with factory defaults or restored EEPROM data.
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access to the storage.
    /// Typically safe during boot before interrupts are enabled.
    pub unsafe fn load_defaults_unchecked(
        &self,
        f: impl FnOnce(&mut WriteFn) -> Result<(), ShadowError>,
    ) -> Result<(), ShadowError> {
        let table = unsafe { &mut *self.table.get() };
        let mut write = |addr: u16, data: &[u8]| table.write_range(addr, data);
        f(&mut write)
    }

    /// Load initial values into the shadow table without marking dirty.
    ///
    /// Wraps [`Self::load_defaults_unchecked`] in a critical section.
    ///
    /// Use this during system initialization to populate the shadow
    /// with factory defaults or restored EEPROM data.
    pub fn load_defaults(
        &self,
        f: impl FnOnce(&mut WriteFn) -> Result<(), ShadowError>,
    ) -> Result<(), ShadowError> {
        critical_section::with(|_| unsafe { self.load_defaults_unchecked(f) })
    }
}
