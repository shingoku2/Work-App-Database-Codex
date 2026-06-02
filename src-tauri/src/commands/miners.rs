use crate::{
    db::DbPool,
    models::{CreateMiner, Miner, MinerImportResult, UpdateMiner},
};
use std::collections::HashSet;
use tauri::State;

const LIST_MINERS_SQL: &str = "SELECT id, serial, model, firmware, client_name, miner_type, ip_address, mac_address, pickaxe, miner_state, miner_row, miner_index, miner_rack, miner_rack_group, location, status, acquired_date, notes FROM miners ORDER BY serial";

const MINER_MODELS: &[&str] = &["S21", "S21+", "S21 Pro", "S21 XP"];
const MINER_STATUSES: &[&str] = &["In Service", "Under Repair", "RMA", "Retired", "Spare"];

/// Result of pre-deduping an import batch by trimmed serial. The DB-bound
/// upsert loop in `import_miners` consumes `to_upsert` and adds `skipped`
/// to its own counter.
pub(crate) struct DedupedBatch {
    pub to_upsert: Vec<CreateMiner>,
    pub skipped: i64,
}

/// Pre-dedup helper: trims each serial and drops empty / duplicate entries.
/// The order of `to_upsert` matches the order of first appearance in the input.
pub(crate) fn dedup_by_serial(miners: Vec<CreateMiner>) -> DedupedBatch {
    let mut seen: HashSet<String> = HashSet::new();
    let mut to_upsert: Vec<CreateMiner> = Vec::with_capacity(miners.len());
    let mut skipped: i64 = 0;
    for miner in miners {
        let trimmed = miner.serial.trim().to_string();
        if trimmed.is_empty() {
            skipped += 1;
            continue;
        }
        if !seen.insert(trimmed) {
            skipped += 1;
            continue;
        }
        to_upsert.push(miner);
    }
    DedupedBatch { to_upsert, skipped }
}

#[tauri::command]
pub async fn list_miners(pool: State<'_, DbPool>) -> Result<Vec<Miner>, String> {
    sqlx::query_as::<_, Miner>(LIST_MINERS_SQL)
    .fetch_all(pool.inner())
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_miner(pool: State<'_, DbPool>, input: CreateMiner) -> Result<i64, String> {
    if input.serial.trim().is_empty() {
        return Err("serial must not be empty".into());
    }
    if !MINER_MODELS.contains(&input.model.as_str()) {
        return Err(format!("model must be one of {MINER_MODELS:?}"));
    }
    if !MINER_STATUSES.contains(&input.status.as_str()) {
        return Err(format!("status must be one of {MINER_STATUSES:?}"));
    }

    let result = sqlx::query(
        r#"
        INSERT INTO miners (
            serial, model, firmware, client_name, miner_type, ip_address, mac_address, pickaxe,
            miner_state, miner_row, miner_index, miner_rack, miner_rack_group, location, status,
            acquired_date, notes
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
        "#,
    )
    .bind(input.serial)
    .bind(input.model)
    .bind(input.firmware)
    .bind(input.client_name)
    .bind(input.miner_type)
    .bind(input.ip_address)
    .bind(input.mac_address)
    .bind(input.pickaxe)
    .bind(input.miner_state)
    .bind(input.miner_row)
    .bind(input.miner_index)
    .bind(input.miner_rack)
    .bind(input.miner_rack_group)
    .bind(input.location)
    .bind(input.status)
    .bind(input.acquired_date)
    .bind(input.notes)
    .execute(pool.inner())
    .await
    .map_err(|error| error.to_string())?;

    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn update_miner(pool: State<'_, DbPool>, input: UpdateMiner) -> Result<(), String> {
    if input.serial.trim().is_empty() {
        return Err("serial must not be empty".into());
    }
    if !MINER_MODELS.contains(&input.model.as_str()) {
        return Err(format!("model must be one of {MINER_MODELS:?}"));
    }
    if !MINER_STATUSES.contains(&input.status.as_str()) {
        return Err(format!("status must be one of {MINER_STATUSES:?}"));
    }

    sqlx::query(
        r#"
        UPDATE miners
        SET serial = ?1, model = ?2, firmware = ?3, client_name = ?4, miner_type = ?5,
            ip_address = ?6, mac_address = ?7, pickaxe = ?8, miner_state = ?9, miner_row = ?10,
            miner_index = ?11, miner_rack = ?12, miner_rack_group = ?13, location = ?14,
            status = ?15, acquired_date = ?16, notes = ?17, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?18
        "#,
    )
    .bind(input.serial)
    .bind(input.model)
    .bind(input.firmware)
    .bind(input.client_name)
    .bind(input.miner_type)
    .bind(input.ip_address)
    .bind(input.mac_address)
    .bind(input.pickaxe)
    .bind(input.miner_state)
    .bind(input.miner_row)
    .bind(input.miner_index)
    .bind(input.miner_rack)
    .bind(input.miner_rack_group)
    .bind(input.location)
    .bind(input.status)
    .bind(input.acquired_date)
    .bind(input.notes)
    .bind(input.id)
    .execute(pool.inner())
    .await
    .map_err(|error| error.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn import_miners(pool: State<'_, DbPool>, miners: Vec<CreateMiner>) -> Result<MinerImportResult, String> {
    let mut tx = pool.begin().await.map_err(|error| error.to_string())?;
    let mut imported = 0i64;
    let mut updated = 0i64;

    let DedupedBatch { to_upsert, skipped } = dedup_by_serial(miners);

    for miner in to_upsert {
        let trimmed = miner.serial.trim().to_string();

        let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM miners WHERE serial = ?1")
            .bind(&trimmed)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;

        if existing.is_some() {
            sqlx::query(
                r#"
                UPDATE miners
                SET model = ?1, firmware = ?2, client_name = ?3, miner_type = ?4,
                    ip_address = ?5, mac_address = ?6, pickaxe = ?7, miner_state = ?8,
                    miner_row = ?9, miner_index = ?10, miner_rack = ?11, miner_rack_group = ?12,
                    location = ?13, status = ?14, acquired_date = ?15, notes = ?16,
                    updated_at = CURRENT_TIMESTAMP
                WHERE serial = ?17
                "#,
            )
            .bind(miner.model)
            .bind(miner.firmware)
            .bind(miner.client_name)
            .bind(miner.miner_type)
            .bind(miner.ip_address)
            .bind(miner.mac_address)
            .bind(miner.pickaxe)
            .bind(miner.miner_state)
            .bind(miner.miner_row)
            .bind(miner.miner_index)
            .bind(miner.miner_rack)
            .bind(miner.miner_rack_group)
            .bind(miner.location)
            .bind(miner.status)
            .bind(miner.acquired_date)
            .bind(miner.notes)
            .bind(&trimmed)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            updated += 1;
        } else {
            sqlx::query(
                r#"
                INSERT INTO miners (
                    serial, model, firmware, client_name, miner_type, ip_address, mac_address, pickaxe,
                    miner_state, miner_row, miner_index, miner_rack, miner_rack_group, location, status,
                    acquired_date, notes
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                "#,
            )
            .bind(&trimmed)
            .bind(miner.model)
            .bind(miner.firmware)
            .bind(miner.client_name)
            .bind(miner.miner_type)
            .bind(miner.ip_address)
            .bind(miner.mac_address)
            .bind(miner.pickaxe)
            .bind(miner.miner_state)
            .bind(miner.miner_row)
            .bind(miner.miner_index)
            .bind(miner.miner_rack)
            .bind(miner.miner_rack_group)
            .bind(miner.location)
            .bind(miner.status)
            .bind(miner.acquired_date)
            .bind(miner.notes)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            imported += 1;
        }
    }

    tx.commit().await.map_err(|error| error.to_string())?;
    Ok(MinerImportResult { imported, updated, skipped })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CreateMiner;

    fn make_miner(serial: &str) -> CreateMiner {
        CreateMiner {
            serial: serial.to_string(),
            model: "S21".to_string(),
            firmware: None,
            client_name: None,
            miner_type: None,
            ip_address: None,
            mac_address: None,
            pickaxe: None,
            miner_state: None,
            miner_row: None,
            miner_index: None,
            miner_rack: None,
            miner_rack_group: None,
            location: None,
            status: "In Service".to_string(),
            acquired_date: None,
            notes: None,
        }
    }

    #[test]
    fn miner_models_contains_each_known_model() {
        for model in ["S21", "S21+", "S21 Pro", "S21 XP"] {
            assert!(
                MINER_MODELS.contains(&model),
                "MINER_MODELS should include {model}"
            );
        }
    }

    #[test]
    fn miner_models_rejects_unknown_model() {
        assert!(!MINER_MODELS.contains(&"S19 Pro"));
        assert!(!MINER_MODELS.contains(&""));
    }

    #[test]
    fn miner_statuses_contains_each_known_status() {
        for status in ["In Service", "Under Repair", "RMA", "Retired", "Spare"] {
            assert!(
                MINER_STATUSES.contains(&status),
                "MINER_STATUSES should include {status}"
            );
        }
    }

    #[test]
    fn miner_statuses_rejects_unknown_status() {
        assert!(!MINER_STATUSES.contains(&"Decommissioned"));
        assert!(!MINER_STATUSES.contains(&"in service")); // case-sensitive
    }

    #[test]
    fn dedup_by_serial_preserves_first_appearance_order() {
        let batch = vec![make_miner("A"), make_miner("B"), make_miner("A")];
        let DedupedBatch { to_upsert, skipped } = dedup_by_serial(batch);
        assert_eq!(to_upsert.len(), 2);
        assert_eq!(to_upsert[0].serial, "A");
        assert_eq!(to_upsert[1].serial, "B");
        assert_eq!(skipped, 1);
    }

    #[test]
    fn dedup_by_serial_skips_empty_serials() {
        let batch = vec![make_miner(""), make_miner("   "), make_miner("A")];
        let DedupedBatch { to_upsert, skipped } = dedup_by_serial(batch);
        assert_eq!(to_upsert.len(), 1);
        assert_eq!(to_upsert[0].serial, "A");
        assert_eq!(skipped, 2);
    }

    #[test]
    fn dedup_by_serial_treats_trimmed_serials_as_duplicates() {
        let batch = vec![make_miner("A"), make_miner("  A  "), make_miner("B")];
        let DedupedBatch { to_upsert, skipped } = dedup_by_serial(batch);
        assert_eq!(to_upsert.len(), 2);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn dedup_by_serial_empty_input_returns_empty() {
        let DedupedBatch { to_upsert, skipped } = dedup_by_serial(vec![]);
        assert!(to_upsert.is_empty());
        assert_eq!(skipped, 0);
    }
}

#[tauri::command]
pub async fn delete_miner(pool: State<'_, DbPool>, id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM miners WHERE id = ?1")
        .bind(id)
        .execute(pool.inner())
        .await
        .map_err(|error| error.to_string())?;

    Ok(())
}
