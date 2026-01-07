use crate::{ShadowError, table::ShadowTable};

pub struct KernelView<'a, const T: usize, const B: usize, const W: usize> {
    table: &'a mut ShadowTable<T, B, W>,
}

impl<'a, const T: usize, const B: usize, const W: usize> KernelView<'a, T, B, W> {
    pub(crate) fn new(table: &'a mut ShadowTable<T, B, W>) -> Self {
        Self { table }
    }

    pub fn read_range(&mut self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError> {
        self.table.read_range(addr, out)
    }

    pub fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError> {
        self.table.write_range(addr, data)
    }

    pub fn is_dirty(&self, addr: u16, len: usize) -> Result<bool, ShadowError> {
        self.table.is_dirty(addr, len)
    }

    pub fn any_dirty(&self) -> bool {
        self.table.any_dirty()
    }

    pub fn clear_dirty(&mut self, addr: u16, len: usize) -> Result<(), ShadowError> {
        self.table.clear_dirty(addr, len)
    }
}
