use clap::ValueEnum;
use fleet_shared::{normalize_and_validate_miner, validate_part, CreateMiner, CreatePart};
use sqlx::{PgPool, Row, SqlitePool};
use std::path::Path;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ConflictPolicy {
    Abort,
    ServerWins,
    ImportWins,
}

pub async fn run(
    postgres: &PgPool,
    path: &Path,
    apply: bool,
    policy: ConflictPolicy,
) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve default enabled site for all imported records.
    let default_site_id: i64 =
        sqlx::query_scalar("SELECT id FROM sites WHERE enabled = TRUE ORDER BY id LIMIT 1")
            .fetch_optional(postgres)
            .await?
            .ok_or("no enabled site found; create a site before importing legacy data")?;

    let url = format!("sqlite://{}?mode=ro", path.to_string_lossy());
    let sqlite = SqlitePool::connect(&url).await?;
    let miner_rows = sqlx::query(
        "SELECT serial, model, firmware, client_name, miner_type, ip_address, mac_address, pickaxe, miner_state, miner_row, miner_index, miner_rack, miner_rack_group, location, status, acquired_date, notes FROM miners ORDER BY serial",
    )
    .fetch_all(&sqlite)
    .await?;
    let part_rows = sqlx::query(
        "SELECT sku, name, category, qty_on_hand, reorder_threshold, supplier, unit_cost, notes FROM parts ORDER BY sku",
    )
    .fetch_all(&sqlite)
    .await?;

    let mut miners = Vec::with_capacity(miner_rows.len());
    for row in miner_rows {
        let mut miner = CreateMiner {
            site_id: Some(default_site_id),
            serial: row.get("serial"),
            model: row.get("model"),
            firmware: row.get("firmware"),
            client_name: row.get("client_name"),
            miner_type: row.get("miner_type"),
            ip_address: row.get("ip_address"),
            mac_address: row.get("mac_address"),
            pickaxe: row.get("pickaxe"),
            miner_state: row.get("miner_state"),
            miner_row: row.get("miner_row"),
            miner_index: row.get("miner_index"),
            miner_rack: row.get("miner_rack"),
            miner_rack_group: row.get("miner_rack_group"),
            location: row.get("location"),
            status: row.get("status"),
            acquired_date: row.get("acquired_date"),
            notes: row.get("notes"),
        };
        normalize_and_validate_miner(&mut miner)?;
        miners.push(miner);
    }
    let mut parts = Vec::with_capacity(part_rows.len());
    for row in part_rows {
        let part = CreatePart {
            site_id: Some(default_site_id),
            sku: row.get("sku"),
            name: row.get("name"),
            category: row.get("category"),
            qty_on_hand: row.get("qty_on_hand"),
            reorder_threshold: row.get("reorder_threshold"),
            supplier: row.get("supplier"),
            unit_cost_cents: dollars_to_cents(row.get("unit_cost"))?,
            notes: row.get("notes"),
        };
        validate_part(&part)?;
        parts.push(part);
    }

    // Conflict checks are site-scoped after migration 0005.
    let miner_conflicts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM miners WHERE site_id = $1 AND serial = ANY($2)")
            .bind(default_site_id)
            .bind(miners.iter().map(|m| m.serial.clone()).collect::<Vec<_>>())
            .fetch_one(postgres)
            .await?;

    let part_conflicts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM parts WHERE site_id = $1 AND sku = ANY($2)")
            .bind(default_site_id)
            .bind(parts.iter().map(|p| p.sku.clone()).collect::<Vec<_>>())
            .fetch_one(postgres)
            .await?;

    println!(
        "SQLite import preview: {} miners ({} conflicts), {} parts ({} conflicts)",
        miners.len(),
        miner_conflicts,
        parts.len(),
        part_conflicts
    );
    if !apply {
        println!("dry run only; pass --apply to write changes");
        return Ok(());
    }
    if matches!(policy, ConflictPolicy::Abort) && miner_conflicts + part_conflicts > 0 {
        return Err(
            "conflicts found; choose --conflict=server-wins or --conflict=import-wins".into(),
        );
    }

    let mut tx = postgres.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;

    for miner in miners {
        // $1=site_id $2=serial $3=model $4=firmware $5=client_name $6=miner_type
        // $7=ip $8=mac $9=pickaxe $10=miner_state $11=miner_row $12=miner_index
        // $13=miner_rack $14=miner_rack_group $15=location $16=status
        // $17=acquired_date $18=notes
        if matches!(policy, ConflictPolicy::ServerWins) {
            sqlx::query(
                "INSERT INTO miners (site_id,serial,model,firmware,client_name,miner_type,ip_address,mac_address,pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,location,status,acquired_date,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) ON CONFLICT (site_id, serial) DO NOTHING",
            )
            .bind(default_site_id)
            .bind(&miner.serial).bind(&miner.model).bind(&miner.firmware).bind(&miner.client_name)
            .bind(&miner.miner_type).bind(&miner.ip_address).bind(&miner.mac_address).bind(&miner.pickaxe)
            .bind(&miner.miner_state).bind(&miner.miner_row).bind(&miner.miner_index).bind(&miner.miner_rack)
            .bind(&miner.miner_rack_group).bind(&miner.location).bind(&miner.status).bind(&miner.acquired_date)
            .bind(&miner.notes).execute(&mut *tx).await?;
        } else if matches!(policy, ConflictPolicy::ImportWins) {
            sqlx::query(
                "INSERT INTO miners (site_id,serial,model,firmware,client_name,miner_type,ip_address,mac_address,pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,location,status,acquired_date,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) ON CONFLICT (site_id, serial) DO UPDATE SET model=EXCLUDED.model, firmware=EXCLUDED.firmware, client_name=EXCLUDED.client_name, miner_type=EXCLUDED.miner_type, ip_address=EXCLUDED.ip_address, mac_address=EXCLUDED.mac_address, pickaxe=EXCLUDED.pickaxe, miner_state=EXCLUDED.miner_state, miner_row=EXCLUDED.miner_row, miner_index=EXCLUDED.miner_index, miner_rack=EXCLUDED.miner_rack, miner_rack_group=EXCLUDED.miner_rack_group, location=EXCLUDED.location, status=EXCLUDED.status, acquired_date=EXCLUDED.acquired_date, notes=EXCLUDED.notes, version=miners.version+1, updated_at=NOW()",
            )
            .bind(default_site_id)
            .bind(&miner.serial).bind(&miner.model).bind(&miner.firmware).bind(&miner.client_name)
            .bind(&miner.miner_type).bind(&miner.ip_address).bind(&miner.mac_address).bind(&miner.pickaxe)
            .bind(&miner.miner_state).bind(&miner.miner_row).bind(&miner.miner_index).bind(&miner.miner_rack)
            .bind(&miner.miner_rack_group).bind(&miner.location).bind(&miner.status).bind(&miner.acquired_date)
            .bind(&miner.notes).execute(&mut *tx).await?;
        } else {
            sqlx::query(
                "INSERT INTO miners (site_id,serial,model,firmware,client_name,miner_type,ip_address,mac_address,pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,location,status,acquired_date,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
            )
            .bind(default_site_id)
            .bind(&miner.serial).bind(&miner.model).bind(&miner.firmware).bind(&miner.client_name)
            .bind(&miner.miner_type).bind(&miner.ip_address).bind(&miner.mac_address).bind(&miner.pickaxe)
            .bind(&miner.miner_state).bind(&miner.miner_row).bind(&miner.miner_index).bind(&miner.miner_rack)
            .bind(&miner.miner_rack_group).bind(&miner.location).bind(&miner.status).bind(&miner.acquired_date)
            .bind(&miner.notes).execute(&mut *tx).await?;
        }
    }

    for part in parts {
        // $1=site_id $2=sku $3=name $4=category $5=qty_on_hand $6=reorder_threshold
        // $7=supplier $8=unit_cost_cents $9=notes
        if matches!(policy, ConflictPolicy::ServerWins) {
            sqlx::query("INSERT INTO parts (site_id,sku,name,category,qty_on_hand,reorder_threshold,supplier,unit_cost_cents,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT (site_id, sku) DO NOTHING")
                .bind(default_site_id)
                .bind(&part.sku).bind(&part.name).bind(&part.category).bind(part.qty_on_hand)
                .bind(part.reorder_threshold).bind(&part.supplier).bind(part.unit_cost_cents).bind(&part.notes)
                .execute(&mut *tx).await?;
        } else if matches!(policy, ConflictPolicy::ImportWins) {
            sqlx::query("INSERT INTO parts (site_id,sku,name,category,qty_on_hand,reorder_threshold,supplier,unit_cost_cents,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT (site_id, sku) DO UPDATE SET name=EXCLUDED.name, category=EXCLUDED.category, qty_on_hand=EXCLUDED.qty_on_hand, reorder_threshold=EXCLUDED.reorder_threshold, supplier=EXCLUDED.supplier, unit_cost_cents=EXCLUDED.unit_cost_cents, notes=EXCLUDED.notes, version=parts.version+1, updated_at=NOW()")
                .bind(default_site_id)
                .bind(&part.sku).bind(&part.name).bind(&part.category).bind(part.qty_on_hand)
                .bind(part.reorder_threshold).bind(&part.supplier).bind(part.unit_cost_cents).bind(&part.notes)
                .execute(&mut *tx).await?;
        } else {
            sqlx::query("INSERT INTO parts (site_id,sku,name,category,qty_on_hand,reorder_threshold,supplier,unit_cost_cents,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)")
                .bind(default_site_id)
                .bind(&part.sku).bind(&part.name).bind(&part.category).bind(part.qty_on_hand)
                .bind(part.reorder_threshold).bind(&part.supplier).bind(part.unit_cost_cents).bind(&part.notes)
                .execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    println!("SQLite import applied");
    Ok(())
}

fn dollars_to_cents(value: f64) -> Result<i64, Box<dyn std::error::Error>> {
    if !value.is_finite() || value < 0.0 || value > i64::MAX as f64 / 100.0 {
        return Err("legacy part unit_cost is outside the supported range".into());
    }
    Ok((value * 100.0).round() as i64)
}
