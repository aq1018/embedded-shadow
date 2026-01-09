pub trait PersistTrigger<PK> {
    fn push_key(&mut self, key: PK);
    fn request_persist(&mut self);
}

pub struct NoPersist;
impl<PK> PersistTrigger<PK> for NoPersist {
    fn push_key(&mut self, _key: PK) {}
    fn request_persist(&mut self) {}
}
