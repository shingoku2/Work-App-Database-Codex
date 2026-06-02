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
    pub updated: i64,
    pub skipped: i64,
}

impl MinerImportResult {
    /// Used by tests to construct a result without going through the DB.
    #[cfg(test)]
    pub fn new(imported: i64, updated: i64, skipped: i64) -> Self {
        Self { imported, updated, skipped }
    }
}

#[derive(Debug, Serialize)]
pub struct DashboardSummary {
    pub unit_count: i64,
    pub part_count: i64,
    pub low_stock_count: i64,
    pub units_by_status: Vec<CountByStatus>,
    pub low_stock_parts: Vec<Part>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn miner_import_result_carries_all_three_counts() {
        let result = MinerImportResult::new(5, 2, 1);
        assert_eq!(result.imported, 5);
        assert_eq!(result.updated, 2);
        assert_eq!(result.skipped, 1);
    }

    #[test]
    fn miner_import_result_serializes_to_expected_field_names() {
        // The TypeScript MinerImportResult in src/features/miners/minerApi.ts
        // destructures exactly these three field names. If a future refactor
        // renames them, this test catches it.
        let result = MinerImportResult::new(1, 2, 3);
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("\"imported\":1"), "json was {json}");
        assert!(json.contains("\"updated\":2"), "json was {json}");
        assert!(json.contains("\"skipped\":3"), "json was {json}");
    }

    #[test]
    fn dashboard_summary_serializes_to_expected_field_names() {
        // The TypeScript DashboardSummary in src/types/db.ts mirrors these
        // field names; a drift here would break the dashboard at runtime.
        let summary = DashboardSummary {
            unit_count: 10,
            part_count: 4,
            low_stock_count: 2,
            units_by_status: vec![],
            low_stock_parts: vec![],
        };
        let json = serde_json::to_string(&summary).expect("serialize");
        for field in ["unit_count", "part_count", "low_stock_count", "units_by_status", "low_stock_parts"] {
            assert!(json.contains(field), "{field} missing from {json}");
        }
    }
}
