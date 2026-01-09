use crate::table::ShadowTable;

pub struct KernelView<'a, const TS: usize, const BS: usize, const BC: usize>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    table: &'a mut ShadowTable<TS, BS, BC>,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize> KernelView<'a, TS, BS, BC>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub(crate) fn new(table: &'a mut ShadowTable<TS, BS, BC>) -> Self {
        Self { table }
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize> KernelView<'a, TS, BS, BC>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), crate::ShadowError> {
        self.table.read_range(addr, out)
    }

    pub fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), crate::ShadowError> {
        self.table.write_range(addr, data)?;
        Ok(())
    }

    pub fn for_each_dirty_block<F>(&self, mut f: F) -> Result<(), crate::ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), crate::ShadowError>,
    {
        self.table.for_each_dirty_block(|addr, data| f(addr, data))
    }

    pub fn is_dirty(&self, addr: u16, len: usize) -> Result<bool, crate::ShadowError> {
        self.table.is_dirty(addr, len)
    }

    pub fn any_dirty(&self) -> bool {
        self.table.any_dirty()
    }

    pub fn clear_dirty(&mut self) {
        self.table.clear_dirty()
    }
}
