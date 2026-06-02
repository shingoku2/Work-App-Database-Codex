use crate::{
    db::DbPool,
    models::{CreatePart, Part},
};
use tauri::State;

#[tauri::command]
pub async fn list_parts(pool: State<'_, DbPool>) -> Result<Vec<Part>, String> {
    sqlx::query_as::<_, Part>(
        "SELECT sku, name, category, qty_on_hand, reorder_threshold, supplier, unit_cost, notes FROM parts ORDER BY name",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_part(pool: State<'_, DbPool>, input: CreatePart) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO parts (sku, name, category, qty_on_hand, reorder_threshold, supplier, unit_cost, notes)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
    )
    .bind(input.sku)
    .bind(input.name)
    .bind(input.category)
    .bind(input.qty_on_hand)
    .bind(input.reorder_threshold)
    .bind(input.supplier)
    .bind(input.unit_cost)
    .bind(input.notes)
    .execute(pool.inner())
    .await
    .map_err(|error| error.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_part(pool: State<'_, DbPool>, input: CreatePart) -> Result<(), String> {
    sqlx::query(
        r#"
        UPDATE parts
        SET name = ?2, category = ?3, qty_on_hand = ?4, reorder_threshold = ?5,
            supplier = ?6, unit_cost = ?7, notes = ?8, updated_at = CURRENT_TIMESTAMP
        WHERE sku = ?1
        "#,
    )
    .bind(input.sku)
    .bind(input.name)
    .bind(input.category)
    .bind(input.qty_on_hand)
    .bind(input.reorder_threshold)
    .bind(input.supplier)
    .bind(input.unit_cost)
    .bind(input.notes)
    .execute(pool.inner())
    .await
    .map_err(|error| error.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_part(pool: State<'_, DbPool>, sku: String) -> Result<(), String> {
    sqlx::query("DELETE FROM parts WHERE sku = ?1")
        .bind(sku)
        .execute(pool.inner())
        .await
        .map_err(|error| error.to_string())?;

    Ok(())
}
