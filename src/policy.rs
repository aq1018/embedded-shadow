pub trait AccessPolicy {
    fn can_read(&self, addr: u16, len: usize) -> bool;
    fn can_write(&self, addr: u16, len: usize) -> bool;
}

#[derive(Default)]
pub struct AllowAllPolicy {}

impl AccessPolicy for AllowAllPolicy {
    fn can_read(&self, _addr: u16, _len: usize) -> bool {
        true
    }

    fn can_write(&self, _addr: u16, _len: usize) -> bool {
        true
    }
}

pub trait PersistPolicy<PK> {
    fn push_persistkeys_for_range<F>(&self, addr: u16, len: usize, push_key: F) -> bool
    where
        F: FnMut(PK);
}

#[derive(Default)]
pub struct NoPersistPolicy {}

impl PersistPolicy<()> for NoPersistPolicy {
    fn push_persistkeys_for_range<F>(&self, _addr: u16, _len: usize, _push_key: F) -> bool {
        false
    }
}
