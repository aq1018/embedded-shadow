/// Receives persistence keys and triggers storage operations.
pub trait PersistTrigger<PK> {
    /// Queues a key identifying data that needs to be persisted.
    fn push_key(&mut self, key: PK);
    /// Signals that queued keys should be persisted to storage.
    fn request_persist(&mut self);
}

/// No-op trigger that discards all persistence requests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoPersist;

impl<PK> PersistTrigger<PK> for NoPersist {
    fn push_key(&mut self, _key: PK) {}
    fn request_persist(&mut self) {}
}
