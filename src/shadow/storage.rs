#![allow(unsafe_code)]

use core::{cell::UnsafeCell, marker::PhantomData};

use crate::shadow::{
    ShadowError,
    handle::{HostShadow, KernelShadow},
    persist::PersistTrigger,
    policy::{AccessPolicy, PersistPolicy},
    table::ShadowTable,
    types::StagingBuffer,
};

/// Marker type for storage without staging support.
pub struct NoStage;

/// Wrapper for storage with staging support.
pub struct WithStage<SB: StagingBuffer> {
    pub(crate) sb: SB,
}

/// Core shadow table storage with configurable policies.
///
/// # Const Generics
/// - `TS`: Total size of the shadow table in bytes
/// - `BS`: Block size in bytes for dirty tracking granularity
/// - `BC`: Block count (must equal `TS / BS`)
///
/// # Type Parameters
/// - `AP`: Access policy controlling read/write permissions
/// - `PP`: Persist policy determining what needs persistence
/// - `PT`: Persist trigger receiving persistence requests
/// - `PK`: Persist key type used to identify regions
/// - `SS`: Stage state (`NoStage` or `WithStage<SB>`)
pub struct ShadowStorageBase<const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SS>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub(crate) table: UnsafeCell<ShadowTable<TS, BS, BC>>,
    pub(crate) access_policy: AP,
    pub(crate) persist_policy: PP,
    pub(crate) persist_trigger: UnsafeCell<PT>,
    pub(crate) stage_state: UnsafeCell<SS>,
    _phantom: PhantomData<PK>,
}

/// Shadow storage without staging support (type alias).
pub type ShadowStorage<const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK> =
    ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, NoStage>;

impl<const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK>
    ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, NoStage>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
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
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
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

#[cfg(test)]
mod tests {
    use crate::shadow::test_support::test_storage;

    #[test]
    fn load_defaults_writes_data_without_marking_dirty() {
        let storage = test_storage();

        storage
            .load_defaults(|write| {
                write(0, &[0x11, 0x22, 0x33, 0x44])?;
                write(32, &[0xAA, 0xBB, 0xCC, 0xDD])?;
                Ok(())
            })
            .unwrap();

        // Verify data was written
        storage.host_shadow().with_view(|view| {
            let mut buf = [0u8; 4];
            view.read_range(0, &mut buf).unwrap();
            assert_eq!(buf, [0x11, 0x22, 0x33, 0x44]);

            view.read_range(32, &mut buf).unwrap();
            assert_eq!(buf, [0xAA, 0xBB, 0xCC, 0xDD]);
        });

        // Verify no dirty flags
        storage.kernel_shadow().with_view(|view| {
            assert!(!view.any_dirty());
        });
    }

    #[test]
    fn load_defaults_multiple_ranges() {
        let storage = test_storage();

        storage
            .load_defaults(|write| {
                for i in 0..4 {
                    let addr = i * 16;
                    write(addr, &[i as u8; 4])?;
                }
                Ok(())
            })
            .unwrap();

        // Verify all ranges written correctly
        storage.host_shadow().with_view(|view| {
            for i in 0..4 {
                let addr = i * 16;
                let mut buf = [0u8; 4];
                view.read_range(addr, &mut buf).unwrap();
                assert_eq!(buf, [i as u8; 4]);
            }
        });
    }

    #[test]
    fn load_defaults_error_propagates() {
        let storage = test_storage();

        let result = storage.load_defaults(|write| {
            write(0, &[0x11; 4])?;
            // Force an error with out-of-bounds write
            write(100, &[0xAA; 4])
        });

        assert!(result.is_err());
    }

    #[test]
    fn normal_writes_work_after_load_defaults() {
        let storage = test_storage();

        // Load defaults
        storage
            .load_defaults(|write| {
                write(0, &[0x11, 0x22, 0x33, 0x44])?;
                Ok(())
            })
            .unwrap();

        // Now do a normal write
        storage.host_shadow().with_view(|view| {
            view.write_range(0, &[0xAA, 0xBB, 0xCC, 0xDD]).unwrap();
        });

        // Should be dirty now
        storage.kernel_shadow().with_view(|view| {
            assert!(view.any_dirty());
            assert!(view.is_dirty(0, 4).unwrap());
        });
    }
}
