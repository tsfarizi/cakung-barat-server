//! Asset database operations

use super::AppState;
use uuid::Uuid;

impl AppState {
    pub async fn get_asset_by_id(
        &self,
        id: &Uuid,
    ) -> Result<Option<crate::asset::models::Asset>, sqlx::Error> {
        sqlx::query_as!(crate::asset::models::Asset, "SELECT id, name, filename, url, description, created_at, updated_at FROM assets WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_all_assets(&self) -> Result<Vec<crate::asset::models::Asset>, sqlx::Error> {
        sqlx::query_as!(crate::asset::models::Asset, "SELECT id, name, filename, url, description, created_at, updated_at FROM assets ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_assets_by_ids(
        &self,
        ids: &Vec<Uuid>,
    ) -> Result<Vec<crate::asset::models::Asset>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_as!(crate::asset::models::Asset, "SELECT id, name, filename, url, description, created_at, updated_at FROM assets WHERE id = ANY($1)", ids)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn insert_asset(
        &self,
        asset: &crate::asset::models::Asset,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO assets (id, name, filename, url, description, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (id) DO UPDATE
             SET name = $2, filename = $3, url = $4, description = $5, updated_at = $7
            "#,
            asset.id,
            &asset.name,
            &asset.filename,
            &asset.url,
            asset.description.as_deref(),
            asset.created_at,
            asset.updated_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_asset(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM assets WHERE id = $1", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
