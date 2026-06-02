use crate::{
    db::DbPool,
    models::{CountByStatus, DashboardSummary, Part},
};
use tauri::State;

#[derive(sqlx::FromRow)]
struct DashboardCounts {
    unit_count: i64,
    part_count: i64,
    low_stock_count: i64,
}

#[tauri::command]
pub async fn get_dashboard_summary(pool: State<'_, DbPool>) -> Result<DashboardSummary, String> {
    let DashboardCounts { unit_count, part_count, low_stock_count } = sqlx::query_as::<_, DashboardCounts>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM miners) AS unit_count,
            (SELECT COUNT(*) FROM parts) AS part_count,
            (SELECT COUNT(*) FROM parts WHERE qty_on_hand <= reorder_threshold) AS low_stock_count
        "#,
    )
    .fetch_one(pool.inner())
    .await
    .map_err(|error| error.to_string())?;

    let units_by_status = sqlx::query_as::<_, CountByStatus>(
        "SELECT status, COUNT(*) AS count FROM miners GROUP BY status ORDER BY status",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|error| error.to_string())?;

    let low_stock_parts = sqlx::query_as::<_, Part>(
        r#"
        SELECT sku, name, category, qty_on_hand, reorder_threshold, supplier, unit_cost, notes
        FROM parts
        WHERE qty_on_hand <= reorder_threshold
        ORDER BY qty_on_hand ASC, name ASC
        LIMIT 10
        "#,
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|error| error.to_string())?;

    Ok(DashboardSummary {
        unit_count,
        part_count,
        low_stock_count,
        units_by_status,
        low_stock_parts,
    })
}
