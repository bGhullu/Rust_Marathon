use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::collections::{HashMap, HashSet, BTreeMap, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex, Semaphore};
use ethers::{prelude::*, types::{Address, Block, Log, H256, U256, Bytes}};
use anyhow::{Result, Context, anyhow};
use tracing::{info, debug, warn, error, instrument};
use once_cell::sync::Lazy;
use prometheus::{IntCounterVec, HistogramVec, IntGaugeVec};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use rayon::prelude::*;
use dashmap::DashMap;
use bloom::{BloomFilter as InternalBloom, ASMS};
use lru::LruCache;


// use crate::{
//     types::{SlotKey, SlotState, SlotDriftEvent, StoragePattern, StorageDelta},
//     cache::AdvancedStateCache,
//     config::ScannerConfig,
//     utils::crypto::{keccak256_optimized, calculate_mapping_slot},
// };

static STORAGE_METRICS: Lazy<HistogramVec> = Lazy::new(|| {
    HistogramVec::new(
        prometheus::new("storage_analysis_duration", "TIme spent analayzing storage changes")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
            &["operation", "contract_type"]
    ).unwrap()
    
});

static DRIFT_DETECTION_ACCURACY: Lazy<IntGaugeVec> = Lazy::new(|| {
    IntGaugeVec::new(
        prometheus::new("drift_detection_accuracy", "Accuracy of drift detection predictions")
        &["prediction", "contract"]
    ).unwrap()
});

static PATTERN_CACHE_HITS: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(
        prometheus::Opts::new("pattern_cache_hits", "Storage pattern cache hits"),
        &["pattern_type"]
    ).unwrap()
});

// Core Data structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SlotKey {
    Cutom(H256),
    BalanceOf(Addresss),
    Reserves(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDriftEvent {
    pub chain: String,
    pub contract: Address,
    pub slot_key: SlotKey,
    pub current_value: H256,
    pub predicted_value: H256,
    pub currecnt_block: u64,
    pub predicted_block: u64,
    pub timestamp: DateTime<Utc>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDelta {
    pub slot_key: SlotKey,
    pub old_value: H256,
    pub new_value: H256,
    pub change_type: StorageChangeType,
    pub impact_score: f64,
    pub confidence: f64,
    pub block_number: u64,
    pub contract: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageChangeType {
    DirectWrite,
    MappingUpdate,
    ArrayPush,
    StructUpdate,
    FlashLoan,
    Arbitrage,
    ReentrancyGuard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLayout {
    pub slots: HashMap<u64, StorageSlotInfo>,
    pub mappings: HashMap<u64, MappingInfo>,
    // pub arrays: HashMap<u64, ArrayInfo>,
    // pub structs: HashMap<u64, StructInfo>,
    pub contract_type: ContractType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotInfo {
    pub slot: u64,
    // pub offset: u8,
    // pub size: u8,
    // pub type_name: String,
    pub semantic_meaning: SlotSemantic,
    // pub access_pattern: AccessPattern,
    pub criticality: CriticalLevel,
    pub typical_change_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingInfo {
    pub base_slot: u64,
    pub key_type: String,
    pub value_type: String,
    pub known_keys: HashSet<H256>,
    pub hot_keys: Vec<H256>, // Fequently accessed keys
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ArrayInfo {
//     pub base_slot: u64,
//     pub element_type: String,
//     pub length_slot: u64,
//     pub max_known_index: u64,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct StructInfo {
//     pub base_slot: u64,
//     pub fields: Vec<StructField>,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct StructField {
//     pub name: String,
//     pub slot_offset: u64,
//     pub byte_offset: u8,
//     pub size: u8,
// }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotSemantic {
    Balance,
    Reserve,
    Price,
    Fee,
    Allowance,
    Ownership,
    Governance,
    // Oracle,
    // Time,
    // Counter,
    // Flag,
    Unknown,
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum AccessPattern {
//     Read,
//     Write,
//     ReadWrite,
//     Atomic,
//     Batch,
//     Sequential,
//     Random,
// }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartiallyOrd, Ord)]
pub enum CriticalLevel {
    Low = 1 ,
    Medium = 2,
    High = 3,
    Critical = 4,
    Emergency = 5,
}

// #[derive(Debug, Clone)]
// pub struct StorageChangeContext {
//     pub contract: Address,
//     pub block_number: u64,
//     pub transaction_index: u64,
//     pub log_index: u64,
//     pub gas_used: U256,
//     pub gas_price: U256,
//     pub caller: Address,
//     pub timestamp: DateTime<Utc>,
// }

// #[derive(Debug, Clone)]
// pub struct AdvancedStorageDelta {
//     pub slot_key:  SlotKey,
//     pub old_value: H256,
//     pub new_value: H256,
//     pub change_type: StorageChangeType,
//     pub impact_score: f64,
//     pub confidence: f64,
//     pub related_slotes: Vec<SlotKey>,
//     pub context: StorageChangeContext,
// }

// #[derive(Debug, Clone, PartialEq)]
// pub enum StorageChangeType {
//     DirectWrite,
//     MappingUpdate,
//     ArrayPush,
//     ArrayPop,
//     StructUpdate,
//     ProxyDelegate,
//     UpgradePattern,
//     ReentrancyGuard,
//     FlashLoan,
//     Arbitrage,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContractType {
    UniswapV2Pair,
    ERC20Token,
    LendingPool,
    Unknown,
}

pub struct SimpleStateCache  {
    slot_value: Arc<RwLock<HashMap<(Address, SlotKey), Vec<H256>>>>,
}



impl SimpleStateCache {
    pub fn new() -> Self {
        Self{
            slot_value:  Arc::new(RwLock::new(HashMap::new())),

        }
    }

    pub async fn store_slot_value(&self, contract: Address, slot: SlotKey, value: H256) {
        let mut cache = self.slot_value.write().await;
        cache.entry((contract, slot)).or_default().push(value);

    // Keep only last 100 values per slot
        if let Some(values) = cache.get_mut(&(contract,slot)) {
            if values.len() > 100 {
                values.drain(0..values.len() -100);
            }
        }
    }

    pub async fn get_slot_history(&self, contract: Address, slot: SlotKey) -> Vec<H256> {
        let cache = self.slot_value.read().await;
        cache.get(&(contract, slot)).cloned().unwrap_or_default()
    }

    pub async fn get_latest_value(&self, contract: Address, slot: SlotKey) -> Option<H256> {
        let cache = self.slot_value.read().await;
        cache.get(&(contract,slot))?.last().cloned()
    }
}

pub struct StorageDriftDetector {
    cache: Arc<SimpleStateCache>,
    contract_layouts: Arc<RwLock<HashMap<Address, StorageLayout>>>,
    drift_history: Arc<RwLock<BTreeMap<u64, Vec<SlotDriftEvent>>>>,
    anomaly_threshold: f64,
}

impl StorageDriftDetector {
    pub fn new ()-> Self {
        Self{
            cache: Arc::new(SimpleStateCache::new()),
            contract_layouts: Arc::new(RwLock::new(HashMap::new())),
            drift_history: Arc::new(RwLock::new(BTreeMap::new())),
            anomaly_threshold: 0.7, // Default threshold for anomaly detection
        }
    }


    /// Main entry point - analyze a block for a storage drifts
    pub async fn analyze_block(&self, block:&Block<H256>, receipts: Vec<TransactionReceipt>)-> Result<Vec<SlotDriftEvent>> {
        let block_number = block.number.ok_or_else (|| anyhow!("Block missing number"))?.as_u64();

        println!("🔍 Analyzing block {} with {} transcations", block_number, receipts.len());

        // Step 1: Extract storage changes from transaction log
        let storage_deltas = self.extract_storage_changes(&recipts,block_numnber).await?;

        // Step 2: Update our cache with new values
        self.update_cache(&storage_deltas).await;

        // Step 3: Detect drift patterns
        let drift_events = self.detect_drift_events(&storage_deltas, block_number).await?;

        // Step 4: Store results
        self.store_drift_events(block_number, &drift_events).await;

        println!("✅ Found {} potential drift events", drift_events.len());

        Ok(drift_events)
    }

    pub async fn extract_storage_changes(&self, receipts: &[TransactionReceipt], block_number: u64) -> Result<Vec<StorageDelta>>{
        let mut deltas = Vec::new();

        for receipt in receipts {
            if let Some(contract_address) = receipt.to{
                // Get or infer storage layour for this contract
                let layout = self.get_storage_layout(contract_address).await;

                // Analyze each log for storage implications
                for log in &recipt.logs{
                    let log_deltas = self.analyse_log(log, &layout, block_number, contract_address).await?;
                    deltas.extend(log_deltas);
                }

            }
        }

        Ok(deltas)
    }  

    async fn analyze_log(&self, log: &Log, layout: &StorageLayout, block_number: u64, contract: Address) -> Result<Vec<StorageDelta>> {
        let mut deltas = Vec::new();

        if log.topics.is_empty() {
            return Ok(deltas);
        }

        let event_signature = log.topics[0];

        // Handle known event types
        match self.classify_event(event_singature) {
            EventType::Transfer => {
                deltas.extend(self.handle_transfer_event(log,layout, block_number, contract).await?);

            },
            EventType::Swap => {
                deltas.extend(self.handle_swap_event(log,layout,block_number,contract).await?);
            },
            EventType::Sync => {
                deltas.extend(self.handle_sync_event(log,layout,block_number,contract).await?);
            }
            EventType::Unknown => {
                // Try generic analysis
                deltas.extend(self.handle_unknow_event(log, layout, block_number, contract).await?);
            }
        }

        Ok(deltas)
    } 

    /// Handle ERC20 Transfer events
    async fn handle_transfer_event(&self, log: &Log, layout: &StorageLayout, block_number: u64, contract: Address) -> Result<Vec<StorageDelta>> {
        let mut deltas = Vec::new();

        if log.topics.len() >= 3 && log.data.len() >=32 {
            let from = Address::from(log.topics[1]);
            let to = Address::from(log.topics[2]);
            let amount = U256::from_big_endian(&log.data[0..32]);

            // Generate balance changes for sender and receiver

            if from != Address::zero(){
                if let Some(old_balance) = self.cache.get_lastest_value(contract, SlotKey::BalanceOf(from)).await {
                   let new_balance = U256::from(old_balance).saturating_sub(amoutn);
                   deltas.push(StorageDelt{
                    slot_key: SlotKey::BalanceOf(from),
                    old_value: old_balance,
                    new_value: H256::from(new_balance),
                    change_type: StorageChangeType::MappingUpdate,
                    impact_score: self.calculate_balance_impact(amount, U256::from(old_balance)),
                    confidence: 0.95,
                    block_number,
                    contract,
                   });
                }
            }
            
            if to != Address::zero(){
                if let Some(old_balance) = self.cache.get_latest_value(contract, SlotKey::BalanceOf(to)).await{
                    let new_balance = U256::from(old_balance).saturating_add(amount);
                    deltas.push(StorageDelt{
                        slot_key: SlotKey::BalanceOf(to),
                        old_value: old_balance,
                        new_valeu: H256::from(new_balance),
                        change_type: StorageChagneType::MappingUpdate,
                        impact_score: self.calculate_balance_impact(amount, U256::from(old_balance)),
                        confidence: 0.95,
                        block_number,
                        contract,
                    });
                }
            }
        }

        Ok(deltas)
    }


}