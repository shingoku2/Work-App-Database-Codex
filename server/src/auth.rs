use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use fleet_shared::{normalize_username, validate_password, User, UserRole};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::io::{self, IsTerminal, Read};

pub fn hash_password(password: &str) -> Result<String, String> {
    validate_password(password)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .and_then(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .ok()
        })
        .is_some()
}

pub fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn resolve_password(password_stdin: bool) -> Result<String, String> {
    if password_stdin {
        let mut password = String::new();
        io::stdin()
            .read_to_string(&mut password)
            .map_err(|error| error.to_string())?;
        let password = password.trim_end_matches(['\r', '\n']).to_string();
        validate_password(&password)?;
        return Ok(password);
    }
    if !io::stdin().is_terminal() {
        return Err("use --password-stdin for non-interactive password input".into());
    }
    let password = rpassword::prompt_password("Password: ").map_err(|error| error.to_string())?;
    validate_password(&password)?;
    Ok(password)
}

pub async fn create_admin(
    pool: &PgPool,
    username: &str,
    display_name: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let username = normalize_username(username);
    let password_hash = hash_password(password)?;
    sqlx::query(
        "INSERT INTO users (username, display_name, password_hash, role) VALUES ($1, $2, $3, 'admin')",
    )
    .bind(username)
    .bind(display_name.trim())
    .bind(password_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn reset_password(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let username = normalize_username(username);
    let password_hash = hash_password(password)?;
    let mut tx = pool.begin().await?;
    let user_id: i64 =
        sqlx::query_scalar("UPDATE users SET password_hash = $1, version = version + 1, updated_at = NOW() WHERE username = $2 RETURNING id")
            .bind(password_hash)
            .bind(username)
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub fn user_from_row(row: &sqlx::postgres::PgRow) -> User {
    User {
        id: row.get("id"),
        site_id: row.try_get("site_id").unwrap_or(None),
        site_name: row.try_get("site_name").unwrap_or(None),
        username: row.get("username"),
        display_name: row.get("display_name"),
        role: if row.get::<String, _>("role") == "admin" {
            UserRole::Admin
        } else {
            UserRole::User
        },
        enabled: row.get("enabled"),
        version: row.get("version"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hashes_verify_and_reject_wrong_passwords() {
        let hash = hash_password("correct-password").expect("hash");
        assert!(verify_password("correct-password", &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn session_tokens_are_random_and_hash_stably() {
        let first = new_token();
        let second = new_token();
        assert_ne!(first, second);
        assert_eq!(token_hash(&first), token_hash(&first));
        assert_ne!(token_hash(&first), first);
    }
}
