use crate::ShadowError;

pub trait HostOps {
    fn read_range(&mut self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError>;
    fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError>;

    fn is_dirty(&self, addr: u16, len: usize) -> Result<bool, ShadowError>;
    fn any_dirty(&self) -> bool;

    #[cfg(feature = "staged")]
    fn read_range_overlay(&mut self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError>;

    #[cfg(feature = "staged")]
    fn action(&mut self) -> Result<(), ShadowError>;

    #[cfg(feature = "staged")]
    fn has_staged(&self) -> bool;
}
