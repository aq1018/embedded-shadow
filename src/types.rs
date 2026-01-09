use crate::ShadowError;

pub trait StagingBuffer {
    fn any_staged(&self) -> bool;
    fn apply_overlay(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError>;
    fn write_staged(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError>;
    fn clear_staged(&mut self) -> Result<(), ShadowError>;
    fn for_each_staged<F>(&self, f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), ShadowError>;
}

pub trait StagingOps {
    fn write_range_staged(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError>;
    fn read_range_overlay(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError>;
    fn clear_staged(&mut self) -> Result<(), ShadowError>;
}
