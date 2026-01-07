pub trait AddressPolicy {
    fn can_read(&self, addr: u16, len: usize) -> bool;
    fn can_write(&self, addr: u16, len: usize) -> bool;
    fn triggers_persist(&self, addr: u16, len: usize) -> bool;
    fn stage_only(&self, addr: u16, len: usize) -> bool;
}

pub struct AllowAllPolicy {}
impl AddressPolicy for AllowAllPolicy {
    fn can_read(&self, _addr: u16, _len: usize) -> bool {
        true
    }

    fn can_write(&self, _addr: u16, _len: usize) -> bool {
        true
    }

    fn triggers_persist(&self, _addr: u16, _len: usize) -> bool {
        false
    }

    fn stage_only(&self, _addr: u16, _len: usize) -> bool {
        false
    }
}
