pub trait PersistTrigger {
    fn request_persist(&self);
}

pub struct NoPersist;
impl PersistTrigger for NoPersist {
    fn request_persist(&self) {}
}
