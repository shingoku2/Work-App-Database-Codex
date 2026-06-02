use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Miner {
    pub id: i64,
    pub serial: String,
    pub model: String,
    pub firmware: Option<String>,
    pub client_name: Option<String>,
    pub miner_type: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub pickaxe: Option<String>,
    pub miner_state: Option<String>,
    pub miner_row: Option<String>,
    pub miner_index: Option<String>,
    pub miner_rack: Option<String>,
    pub miner_rack_group: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub acquired_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMiner {
    pub serial: String,
    pub model: String,
    pub firmware: Option<String>,
    pub client_name: Option<String>,
    pub miner_type: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub pickaxe: Option<String>,
    pub miner_state: Option<String>,
    pub miner_row: Option<String>,
    pub miner_index: Option<String>,
    pub miner_rack: Option<String>,
    pub miner_rack_group: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub acquired_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMiner {
    pub id: i64,
    pub serial: String,
    pub model: String,
    pub firmware: Option<String>,
    pub client_name: Option<String>,
    pub miner_type: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub pickaxe: Option<String>,
    pub miner_state: Option<String>,
    pub miner_row: Option<String>,
    pub miner_index: Option<String>,
    pub miner_rack: Option<String>,
    pub miner_rack_group: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub acquired_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Part {
    pub sku: String,
    pub name: String,
    pub category: String,
    pub qty_on_hand: i64,
    pub reorder_threshold: i64,
    pub supplier: Option<String>,
    pub unit_cost: f64,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePart {
    pub sku: String,
    pub name: String,
    pub category: String,
    pub qty_on_hand: i64,
    pub reorder_threshold: i64,
    pub supplier: Option<String>,
    pub unit_cost: f64,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CountByStatus {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct MinerImportResult {
    pub imported: i64,
}

#[derive(Debug, Serialize)]
pub struct DashboardSummary {
    pub unit_count: i64,
    pub part_count: i64,
    pub low_stock_count: i64,
    pub units_by_status: Vec<CountByStatus>,
    pub low_stock_parts: Vec<Part>,
}
