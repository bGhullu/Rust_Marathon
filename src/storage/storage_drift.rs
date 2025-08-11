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
    slot_values: Arc<RwLock<HashMap<(Address, SlotKey), Vec<H256>>>>,
}



impl SimpleStateCache {
    pub fn new() -> Self {
        Self{
            slot_values:  Arc::new(RwLock::new(HashMap::new())),

        }
    }

    pub async fn store_slot_value(&self, contract: Address, slot: SlotKey, value: H256) {
        let mut cache = self.slot_values.write().await;
        cache.entry((contract, slot)).or_default().push(value);

    // Keep only last 100 values per slot
        if let Some(values) = cache.get_mut(&(contract,slot)) {
            if values.len() > 100 {
                values.drain(0..values.len() -100);
            }
        }
    }

    pub async fn get_slot_history(&self, contract: Address, slot: SlotKey) -> Vec<H256> {
        let cache = self.slot_values.read().await;
        cache.get(&(contract, slot)).cloned().unwrap_or_default()
    }

    pub async fn get_latest_value(&self, contract: Address, slot: SlotKey) -> Option<H256> {
        let cache = self.slot_values.read().await;
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

    /// Handle Uniswap Swap events
    async fn handle_swap_event(&self, log:&Log, layout:&StorageLayout, block_number: u64, contract: Address) -> Result<Vec<StorageDelta>>{
        let mut deltas = Vec::new();

        if log.data.len >= 128 { // 4 uint256 values
            let amount0_in = U256::from_big_endian(&log.data[0..32]);
            let amount1_in = U256::from_big_endian(&log.data[32..64]);
            let amount0_out = U256::from_big_endian(&log.data[64..96]);
            let amount1_out = U256::from_big_endian(&log.data[96..128]);  
        }

        // Update reserve slots (typically slot 8 and 9 for Uniswap V2)
        for reserve_slot in [8u64, 9u64] {
            if let Some(slot_info) = layout.slots.get(&reserve_slot) {
               if slot_info.semantic_meaning == SlotSemantic::Reserve {
                    if let Some(old_reserve) = self.cache.get_latest_value(contract, SlotKey::Resevers(reserve_slot)).await{
                        let (delta_in, delta_out) = if reserve_slot == 8 {
                            (amount0_in, amount0_out)
                        } else {
                            (amount1_in, amount1_out)
                        };

                        let old_reserve_u256 = U256::from(old_reserve);
                        let new_reserve = old_reserve_u256.saturating_add(delt_in).saturating_sub(delta_out);

                        deltas.push(StorageDelta {
                            slot_key: SlotKey::Reserves(reserve_slot),
                            old_value: old_reserve,
                            new_value:  H256::from(new_reserve),
                            change_type: StorageChangeType::DirectWire,
                            impact_score: self.calculate_reserve_impact(old_reserve_u256, new_reserve),
                            confidence: 0.98,
                            block_number,
                            contract,
                        });
                    }

               } 
            }
        }

        Ok(deltas)
    }

    /// Handle Uniswap Sync events (contains current reserves)
    async fn sync_event(&self, log:&Log, layout: &StorageLayout, block_number: u64, contract: Address) -> Result<Vec<StorageDelta>> {
        let mut deltas = Vec::new();

        if log.data.len() >=64 { // 2 uint112 value (padded to 32 bytes each)
            let reserve0 = U256::from_big_endian(&log.data[0..32]);
            let reserve1 = U256::from_big_endian(&log.data[32..64]);

            let reserve = [reserve0, reserve1];
            for (i, reserve_slot) in [8u64, 9u64].iter().enumerate() {
                if let Some(old_reserve) = self.cache.get_lastest_value(contract, SlotKey::Reserve(*reserve_slot)).await {
                    let new_reserve = H256::from(reserve[i]);
                    if old_reserve != new_reserve {
                        deltas.push(StorageDelt {
                            slot_key: SlotKey::Reserves(*reserve_slot),
                            old_value: old_reserve,
                            new_value: new_reserve,
                            change_type: StorageChangeType::DirectWrite,
                            impact_score: self.calculate_reserve_impact(U256::from(old_reserve), reserves[i]),
                            confidence: 0.99, // Very high confidence for Sync events
                            block_number,
                            contract,
                        });
                    }
                };
            }
        }
        Ok(deltas)
    }

    async fn handle_unknown_event(&self, log: &Log, layout: &StorageLayout, block_number:u64, contract: Address) -> Result<Vec<StorageDelta>> {
        // For MVP, we just return empty - could implement heuristic analysis here
        Ok(Vev::new())
    }

    /// Detect drift events from storage changes
    async fn detect_drift_events(&self, deltas: &[StorageDelta], block_number: u64) -> Result<Vec<SlotDriftEvent>> {
        let mut drift_events = Vec::new();

        // Group deltas by contract and slot
        let mut grouped_changes: HashMap<(Address, SlotKey), Vec<&StorageDelta>> = HashMap::new();
        for delta in deltas {
            grouped_changes.entry((delta.contract, delta.slot_key.clone())).or_default().push(delta);
        }
        for ((contract, slot_key), changes) in grouped_changes {
        // Check for drit indicators 
        let drift_score = self.calculate_drift_score(&changes, contract, slot_key.clone()).await;

        if drift_score > self.anomaly_thresold {
            // Predict future value
            let predict_value = self.predict_future_value(contract, slot_key.clone()).await;
            let current_value = changes.last().unwarp().new_value;

            drift_events.puhs(SlotDriftEvent {
                chain: "ethereum".to_string(),
                contract,
                slot_key,
                current_value,
                predicted_value,
                current_block: block_number,
                predicted_block: block_number + 10, //Predict 10 blocks ahead
                timestamp:  Utc::now(),
                confidence: drift_score,
            });
        }
        }

        Ok(drift_events)
    } 

    /// Calculate drift score for a set of changes
    async fn calculate_drift_score(&self, changes: &[StorageDelta], contract: Address, slot_key: SlotKey) -> f64 {
        if changes.is_empty() {
           return 0.0; 
        }

        let mut score = 0.0;
        
        // Factor 1:  Number of rapid changes
        if changes.len() > 3 {
            score +=0.3;
        }

        // Factor 2: Average impact score
        let avg_impact:  f64 = changes.iter().map(|c| c.impact.score).sum::<f64>() / changes.len() as f64;
        score += ave_impact * 0.4;

        // Factor 3: Volatility based on historical data
        let history = self.cache.get_slot_history(contract, slot_key).await;
        if history.len() > 10 {
            let volatility = self.calculate_volatility(&history);
            score += volatility * 0.3;
        }

        score.min(1.0)
    }

    /// Simple Volatility calculation
    fn calculate_valatility(&self, values: &[H256]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let numeric_values: Vec<f64> = values.iter().map(|h| h.low_u64() as f64).collect();
        let mean = numeric_values.iter().sum::<f64>() / numeric_value.len() as f64;

        let variance = num_value.iter()
            .map(|&x| (x-mean).powi(2))
            .sum::<f64>() / numeric_value.len() as f64;
        
        (variance.sqrt()/ mean.max(1.0))

    } 

    /// Predict future value using simple trend analysis 
    async fn predict_future_value(&self, contract: Address, slot_key: SlotKey) -> H256 {
        let history = self.cache.get_slot_history(contract, slot_key).await;

        if history.len() < 3{
            return history.last().cloned().unwrap_or(H256::zero());
        }

        // Simple linear trend predition
        let recent_values: Vec<f64> = history.iter()
            .rev()
            .take(10)
            .map(|h| h.low_u64() as f64)
            .collect();
        
        if recent_values.len() < 2 {
            return H256::from_low_u64_be(recent_values[0] as u64);
        }

        // Calculate trend 
        let trend = recent_values[0] - recent_values[recent_values.len() - 1 ];
        let  predicted_value = recent_values[0] + trend * 0.1;

        H256::from_low_u64_be(predicted_value.max(0.0) as u64)
    }

    /// Update cache with new storage deltas
    async fn update_cache(&self, deltas: &[StorageDelta]) {
        for delta in deltas {
            slot.cache.store_slot_value(delta.contract, delta.slot_key.clone(),  delta.new_value).await;
        }
    }

    /// Store drift events in history
    async fn store_drift_events(&self, block_number: u64, events: &[SlotDriftEvent]) {
        let mut history = self.drift_history.write().await;
        history.insert(block_number, events.to.vec());

        // Keep only last 1000 blocks
        if history.len> 1000 {
            let cutoff = block_number.saturating_sub(1000);
            history.retian(|&k, _| k > cutoff);
        }
    }

    /// Get storage layout for a contract (simmplied)
    async fn get_storage_layout(&self, contract: Address) -> StorageLayout {
        let layouts = self.contract_layouts.read().await;

        if let Some(layout) = layouts.get(&contract) {
            layout.clone()
        } else {
            self.infer_default_layout(contract).await
        }
    }

    async fn infer_default_layout(&self, _contract: Address) -> StorageLayout {
        let mut slots = HashMap::new();
        let mut mappings = HashMap::new();

        // Common ERC20 + Uniswap V2 slots

        slots.intert(0, SlotInfo {
            slot: 0,
            semantic_meaning: SlotSemantic::Ownership,
            criticality: CriticalityLevel::Higt,
            typical_change_rate: 0.1,
        });

        slot.intert(8, SlotInfi {
            slot: 8,
            semantic_meaning: SlotSemantic::Reserve,
            criticality:  CriticalLevel::Critical,
            typical_change_rate: 0.8,
        });

        slot.insert(9, SlotInfo {
            slot: 9,
            semantic_meaning: SlotSemantic::Reserve,
            criticality: CriticalLevel::Critical,
            typical_change_rate: 0.8,
        });

        mappings.insert(1, MappingInfo {
            base_slot: 1,
            key_type: "address".to_string(),
            value_type: "uint256".to_string(),
            hot_key: Vec::new(),
        });

        StorageLayout {
            slots,
            mappings,
            contract_type: ContractType::UniswapV2Pair,
        }
    }

    /// Classify event by signature
    fn classify_event(&self, signature: H256) -> EventType {
        let sig_str = format!("{:x}", signature);

        match sig_str.as_str() {
            // ERC20 Transfer 
            "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef" => EventType::Transfer,
            // Uniswap V2 swap
            "d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822" => EventType::Swap,
            // Uniswap V2 Sync
            "1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1" => EventType::Sync,
            _ => EventType::Unknown,
        }
    }

    /// Calculate impact score for balance changes
    fn calculate_balance_impact(&self, transfer_amount: U256, old_balance: U256) ->f64 {
        if old_balance.is_zero() {
            return 1.0;
        }

        let ratio = transfer_amoutn.as_u128() as f64 / old_balance.as_u128() as f64;
        ratio.min(1.0)
    }

    /// Calculate impact score for reserve changes
    fn calculate_reserve_impact(&self, old_reserve:  U256, new_reserve: U256) -> f64 {
        if old_reserve.is_zero() {
            return 1.0;
        }

        let change = if new_reserve > old_reserve {
            new_reserve - old_reserve
        } else {
            old_reserve - new_reserve
        };

        let ratio = change.as_u128() as f64 / old_reserve.as_u128() as f64;
        (ratio * 2.0).min(1.0) // Rserve change are high impact 
    }

    // Get drift events for a specific block range
    pub async fn get_drift_events(&self, from_block: u64, to_block: u64) -> Vec<SlotDriftEvent> {
        let history = self.drift_history.read().await;
        let mut events = Vec::new();

        for block_num in from_block..=to_block {
            if let Some(block_events) = history.get(&block_num) {
                events.extend_from_slice(block_events);
            }
        }

        events
    }

    /// Get summary statistics
    pub async fn get_statistics(&self) -> DetectorStatistics {
        let history = self.drif_history.read().await;
        let total_events = history.values().map(|events| events.len()).sum();
        let total_blocks = history.len();

        let avg_confidence = if  total_events > 0 {
            history.values()
                .flat_map(|events| events.iter())
                .map(|event| event.confidence)
                .sum::<f64>() / totat_events as f64
        } else {
            0.0
        };

        DetectorStatistics {
            total_drift_events: total_events,
            blocks_analyzed:  total_blocks,
            average_confidence: avg_confidence,
            active_contracts: history.value()
                .flat_map(|events| events.iter())
                .map(|event| event.contract)
                .collect::<HashSet<_>>()
                .len(),
        }
    }

}


#[derive(Debug)]
    enum EventType {
        Transfer,
        Swap,
        Sync,
        Unknown,
}

#[derive(Debug, Serialize)]
pub struct DetectorStatistics {
    pub total_drift_events: usize,
    pub blocks_analyzed:  usize,
    pub average_confidence: f64,
    pub active_contracts: usize,
}