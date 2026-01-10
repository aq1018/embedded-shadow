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
        let mut write = |addr: u16, data: &[u8]| {
            table.with_bytes_mut(addr, data.len(), |buf| {
                buf.copy_from_slice(data);
                Ok(())
            })
        };
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
    use crate::shadow::{WriteResult, test_support::test_storage};

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
            view.with_ro_slice(0, 4, |slice| {
                let mut buf = [0u8; 4];
                slice.copy_to_slice(&mut buf);
                assert_eq!(buf, [0x11, 0x22, 0x33, 0x44]);
            })
            .unwrap();

            view.with_ro_slice(32, 4, |slice| {
                let mut buf = [0u8; 4];
                slice.copy_to_slice(&mut buf);
                assert_eq!(buf, [0xAA, 0xBB, 0xCC, 0xDD]);
            })
            .unwrap();
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
                view.with_ro_slice(addr, 4, |slice| {
                    let mut buf = [0u8; 4];
                    slice.copy_to_slice(&mut buf);
                    assert_eq!(buf, [i as u8; 4]);
                })
                .unwrap();
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
            view.with_wo_slice(0, 4, |mut slice| {
                slice.copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
                WriteResult::Dirty(())
            })
            .unwrap();
        });

        // Should be dirty now
        storage.kernel_shadow().with_view(|view| {
            assert!(view.any_dirty());
            assert!(view.is_dirty(0, 4).unwrap());
        });
    }

    #[test]
    fn full_host_kernel_sync_cycle() {
        let storage = test_storage();

        // 1. Host writes to addr 0 and 32 -> marks dirty
        storage.host_shadow().with_view(|view| {
            view.with_wo_slice(0, 4, |mut slice| {
                slice.copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
                WriteResult::Dirty(())
            })
            .unwrap();
            view.with_wo_slice(32, 4, |mut slice| {
                slice.copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
                WriteResult::Dirty(())
            })
            .unwrap();
        });

        // 2. Kernel iter_dirty sees both blocks
        storage.kernel_shadow().with_view(|view| {
            let mut dirty_addrs = [0u16; 4];
            let mut count = 0;
            view.iter_dirty(|addr, _data| {
                dirty_addrs[count] = addr;
                count += 1;
                Ok(())
            })
            .unwrap();
            assert_eq!(count, 2);
            assert_eq!(dirty_addrs[0], 0);
            assert_eq!(dirty_addrs[1], 32);
        });

        // 3. Kernel clears block 0 only
        storage.kernel_shadow().with_view(|view| {
            view.clear_dirty(0, 16).unwrap();
        });

        // 4. Host writes to addr 48
        storage.host_shadow().with_view(|view| {
            view.with_wo_slice(48, 4, |mut slice| {
                slice.copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
                WriteResult::Dirty(())
            })
            .unwrap();
        });

        // 5. Kernel iter_dirty sees blocks 2 (addr 32) and 3 (addr 48), but not 0
        storage.kernel_shadow().with_view(|view| {
            let mut dirty_addrs = [0u16; 4];
            let mut count = 0;
            view.iter_dirty(|addr, _data| {
                dirty_addrs[count] = addr;
                count += 1;
                Ok(())
            })
            .unwrap();
            assert_eq!(count, 2);
            assert_eq!(dirty_addrs[0], 32);
            assert_eq!(dirty_addrs[1], 48);

            // Verify block 0 is NOT dirty
            assert!(!view.is_dirty(0, 16).unwrap());
        });
    }
}
