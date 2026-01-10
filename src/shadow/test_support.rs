//! Test support utilities - only compiled in test builds.

use super::persist::NoPersist;
use super::policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy};
use super::staged::PatchStagingBuffer;
use super::storage::ShadowStorage;
use super::table::ShadowTable;

/// Standard test configuration: 64 bytes, 16-byte blocks, 4 blocks
pub type TestTable = ShadowTable<64, 16, 4>;
pub type TestStorage = ShadowStorage<64, 16, 4, AllowAllPolicy, NoPersistPolicy, NoPersist, ()>;
pub type TestStage = PatchStagingBuffer<64, 8>;

/// Policy that denies all access
pub struct DenyAllPolicy;

impl AccessPolicy for DenyAllPolicy {
    fn can_read(&self, _addr: u16, _len: usize) -> bool {
        false
    }
    fn can_write(&self, _addr: u16, _len: usize) -> bool {
        false
    }
}

/// Policy that allows reads everywhere but only writes at addr >= 32
pub struct ReadOnlyBelow32;

impl AccessPolicy for ReadOnlyBelow32 {
    fn can_read(&self, _addr: u16, _len: usize) -> bool {
        true
    }
    fn can_write(&self, addr: u16, len: usize) -> bool {
        addr >= 32 && (addr as usize + len) <= 64
    }
}

/// Helper to create a default test storage
pub fn test_storage() -> TestStorage {
    ShadowStorage::new(
        AllowAllPolicy::default(),
        NoPersistPolicy::default(),
        NoPersist,
    )
}
