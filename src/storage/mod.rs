mod storage_drift;

pub use storage_drift::{
    StorageDriftDetector, SlotDriftEvent,  SlotKey, StorageDelta,
    StorageChangeType, SlotSemantic, CriticalLevel, SimpleStateCache,
};