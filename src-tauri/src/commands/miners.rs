use crate::{
    db::DbPool,
    models::{CreateMiner, Miner, MinerImportResult, UpdateMiner},
};
use tauri::State;

const MINER_SELECT: &str = "SELECT id, serial, model, firmware, client_name, miner_type, ip_address, mac_address, pickaxe, miner_state, miner_row, miner_index, miner_rack, miner_rack_group, location, status, acquired_date, notes FROM miners";

#[tauri::command]
pub async fn list_miners(pool: State<'_, DbPool>) -> Result<Vec<Miner>, String> {
    sqlx::query_as::<_, Miner>(&format!("{MINER_SELECT} ORDER BY serial"))
    .fetch_all(pool.inner())
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_miner(pool: State<'_, DbPool>, input: CreateMiner) -> Result<i64, String> {
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
    let mut imported = 0;

    for miner in miners {
        if miner.serial.trim().is_empty() {
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO miners (
                serial, model, firmware, client_name, miner_type, ip_address, mac_address, pickaxe,
                miner_state, miner_row, miner_index, miner_rack, miner_rack_group, location, status,
                acquired_date, notes
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(serial) DO UPDATE SET
                model = excluded.model,
                firmware = excluded.firmware,
                client_name = excluded.client_name,
                miner_type = excluded.miner_type,
                ip_address = excluded.ip_address,
                mac_address = excluded.mac_address,
                pickaxe = excluded.pickaxe,
                miner_state = excluded.miner_state,
                miner_row = excluded.miner_row,
                miner_index = excluded.miner_index,
                miner_rack = excluded.miner_rack,
                miner_rack_group = excluded.miner_rack_group,
                location = excluded.location,
                status = excluded.status,
                acquired_date = excluded.acquired_date,
                notes = excluded.notes,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(miner.serial.trim().to_string())
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

    tx.commit().await.map_err(|error| error.to_string())?;
    Ok(MinerImportResult { imported })
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
