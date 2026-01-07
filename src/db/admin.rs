//! Admin database operations for authentication

use super::AppState;
use uuid::Uuid;

impl AppState {
    /// Get count of admins in database
    pub async fn get_admin_count(&self) -> Result<i64, sqlx::Error> {
        let result = sqlx::query_scalar!("SELECT COUNT(*) FROM admins")
            .fetch_one(&self.pool)
            .await?;
        Ok(result.unwrap_or(0))
    }

    /// Get admin by username
    pub async fn get_admin_by_username(
        &self,
        username: &str,
    ) -> Result<Option<crate::auth::model::Admin>, sqlx::Error> {
        sqlx::query_as!(
            crate::auth::model::Admin,
            "SELECT id, username, password_hash, display_name, refresh_token, created_at, updated_at, created_by FROM admins WHERE username = $1",
            username
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Get admin by refresh token
    pub async fn get_admin_by_refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<Option<crate::auth::model::Admin>, sqlx::Error> {
        sqlx::query_as!(
            crate::auth::model::Admin,
            "SELECT id, username, password_hash, display_name, refresh_token, created_at, updated_at, created_by FROM admins WHERE refresh_token = $1",
            refresh_token
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Create new admin
    pub async fn create_admin(
        &self,
        username: &str,
        password_hash: &str,
        display_name: Option<&str>,
        created_by: Option<Uuid>,
    ) -> Result<crate::auth::model::Admin, sqlx::Error> {
        sqlx::query_as!(
            crate::auth::model::Admin,
            r#"
            INSERT INTO admins (username, password_hash, display_name, created_by)
            VALUES ($1, $2, $3, $4)
            RETURNING id, username, password_hash, display_name, refresh_token, created_at, updated_at, created_by
            "#,
            username,
            password_hash,
            display_name,
            created_by
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Update admin's refresh token (invalidates previous sessions)
    pub async fn update_admin_refresh_token(
        &self,
        admin_id: &Uuid,
        refresh_token: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE admins SET refresh_token = $1, updated_at = NOW() WHERE id = $2",
            refresh_token,
            admin_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get all admins
    pub async fn get_all_admins(&self) -> Result<Vec<crate::auth::model::Admin>, sqlx::Error> {
        sqlx::query_as!(
            crate::auth::model::Admin,
            "SELECT id, username, password_hash, display_name, refresh_token, created_at, updated_at, created_by FROM admins ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Delete admin by id
    pub async fn delete_admin(&self, admin_id: &Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM admins WHERE id = $1", admin_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running database with the admins table
    // Run with: cargo test --test '*' -- --ignored

    #[tokio::test]
    #[ignore = "requires database connection"]
    async fn test_admin_count_empty() {
        // This test would need a mock or test database
        // Placeholder for integration test
    }

    #[test]
    fn test_admin_model_clone() {
        let admin = crate::auth::model::Admin {
            id: Uuid::new_v4(),
            username: "test".to_string(),
            password_hash: "hash".to_string(),
            display_name: Some("Test User".to_string()),
            refresh_token: None,
            created_at: None,
            updated_at: None,
            created_by: None,
        };

        let cloned = admin.clone();
        assert_eq!(admin.id, cloned.id);
        assert_eq!(admin.username, cloned.username);
    }
}
