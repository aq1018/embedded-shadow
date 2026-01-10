/// Controls read/write access to shadow table regions.
pub trait AccessPolicy {
    /// Returns true if reading from `addr` for `len` bytes is allowed.
    fn can_read(&self, addr: u16, len: usize) -> bool;
    /// Returns true if writing to `addr` for `len` bytes is allowed.
    fn can_write(&self, addr: u16, len: usize) -> bool;
}

/// Default policy that allows all reads and writes.
#[derive(Debug, Default, Clone, Copy)]
pub struct AllowAllPolicy {}

impl AccessPolicy for AllowAllPolicy {
    fn can_read(&self, _addr: u16, _len: usize) -> bool {
        true
    }

    fn can_write(&self, _addr: u16, _len: usize) -> bool {
        true
    }
}

/// Determines which regions require persistence and emits keys for them.
pub trait PersistPolicy<PK> {
    /// Pushes persistence keys for the given range and returns true if persistence is needed.
    fn push_persist_keys_for_range<F>(&self, addr: u16, len: usize, push_key: F) -> bool
    where
        F: FnMut(PK);
}

/// Default policy that never triggers persistence.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoPersistPolicy {}

impl PersistPolicy<()> for NoPersistPolicy {
    fn push_persist_keys_for_range<F>(&self, _addr: u16, _len: usize, _push_key: F) -> bool {
        false
    }
}
