use dotenvy::dotenv;
use serde::{Serialize, de::DeserializeOwned};
use std::env;
use std::sync::Arc;
use tokio_postgres::NoTls;
use uuid::Uuid;
use serde_json::Value;
use log;

pub struct AppState {
    pub client: Arc<tokio_postgres::Client>,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();
        
        let database_url = env::var("SUPABASE_DATABASE_URL")
            .unwrap_or_else(|_| panic!("SUPABASE_DATABASE_URL must be set"));

        let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
        

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {}", e);
            }
        });

        Ok(AppState {
            client: Arc::new(client),
        })
    }

    pub async fn get_item<T: DeserializeOwned>(
        &self,
        table_name: &str,
        key: &Uuid,
    ) -> Result<Option<T>, Box<dyn std::error::Error>> {
        log::debug!("Attempting to retrieve item with ID: {} from table: {}", key, table_name);
        let query = format!("SELECT * FROM {} WHERE id = $1", table_name);
        let rows = self.client.query(&query, &[key]).await?;
        
        if let Some(row) = rows.get(0) {
            log::debug!("Found item with ID: {} in table: {}", key, table_name);
            let json_value = row_to_json(row);
            let item: T = serde_json::from_value(json_value)?;
            log::debug!("Successfully deserialized item with ID: {}", key);
            Ok(Some(item))
        } else {
            log::debug!("Item with ID: {} not found in table: {}", key, table_name);
            Ok(None)
        }
    }

    pub async fn get_all_items<T: DeserializeOwned>(
        &self,
        table_name: &str,
    ) -> Result<Vec<T>, Box<dyn std::error::Error>> {
        log::debug!("Attempting to retrieve all items from table: {}", table_name);
        let query = format!("SELECT * FROM {}", table_name);
        let rows = self.client.query(&query, &[]).await?;
        
        let mut items = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let json_value = row_to_json(&row);
            let item: T = serde_json::from_value(json_value)?;
            items.push(item);
            log::trace!("Retrieved item {} from table: {}", index, table_name);
        }
        
        log::info!("Successfully retrieved {} items from table: {}", items.len(), table_name);
        Ok(items)
    }

    pub async fn insert_item<T: Serialize>(
        &self,
        table_name: &str,
        _key: &Uuid,
        item: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting to insert item into table: {}", table_name);
        let json_value = serde_json::to_value(item)?;
        

        let query = match table_name {
            "assets" => {
                "INSERT INTO assets (id, name, filename, url, description, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, NOW(), NOW()) ON CONFLICT (id) DO UPDATE SET name = $2, filename = $3, url = $4, description = $5, updated_at = NOW()"
            },
            "postings" => {
                "INSERT INTO postings (id, judul, tanggal, detail, created_at, updated_at) VALUES ($1, $2, $3, $4, NOW(), NOW()) ON CONFLICT (id) DO UPDATE SET judul = $2, tanggal = $3, detail = $4, updated_at = NOW()"
            },
            _ => {
                log::error!("Unsupported table for insert_item: {}", table_name);
                return Err("Unsupported table for insert_item".into());
            }
        };
        
        match table_name {
            "assets" => {
                let asset: crate::asset::models::Asset = serde_json::from_value(json_value)?;
                log::debug!("Inserting asset with ID: {} and name: {}", asset.id, asset.name);
                self.client.execute(
                    query,
                    &[&asset.id, &asset.name, &asset.filename, &asset.url, &asset.description.as_ref().map(|s| s.as_str())],
                ).await?;
                log::info!("Successfully inserted/updated asset with ID: {}", asset.id);
            },
            "postings" => {
                let posting: crate::posting::models::Posting = serde_json::from_value(json_value)?;
                log::debug!("Inserting posting with ID: {} and title: {}", posting.id, posting.judul);
                self.client.execute(
                    query,
                    &[&posting.id, &posting.judul, &posting.tanggal, &posting.detail],
                ).await?;
                log::info!("Successfully inserted/updated posting with ID: {}", posting.id);
            },
            _ => {
                return Err("Unsupported table for insert_item".into());
            }
        };
        
        log::debug!("Completed insert_item operation for table: {}", table_name);
        Ok(())
    }

    pub async fn delete_item(
        &self,
        table_name: &str,
        key: &Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting to delete item with ID: {} from table: {}", key, table_name);
        let query = format!("DELETE FROM {} WHERE id = $1", table_name);
        let result = self.client.execute(&query, &[key]).await?;
        log::info!("Successfully deleted {} rows from table: {} with ID: {}", result, table_name, key);
        Ok(())
    }

    pub async fn upsert_posting_with_assets(
        &self,
        posting: &crate::posting::models::Posting,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting to upsert posting with ID: {} and {} assets", posting.id, posting.asset_ids.len());
        
        // Insert or update the posting in the postings table
        let query = "INSERT INTO postings (id, judul, tanggal, detail, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO UPDATE SET judul = $2, tanggal = $3, detail = $4, updated_at = $6";
        self.client.execute(
            query,
            &[
                &posting.id,
                &posting.judul,
                &posting.tanggal,
                &posting.detail,
                &posting.created_at.unwrap_or_else(|| chrono::Utc::now()),
                &posting.updated_at.unwrap_or_else(|| chrono::Utc::now()),
            ],
        ).await?;
        
        // Delete existing posting-asset relationships
        let delete_query = "DELETE FROM posting_assets WHERE posting_id = $1";
        self.client.execute(delete_query, &[&posting.id]).await?;
        
        // Insert new posting-asset relationships
        for asset_id in &posting.asset_ids {
            let insert_query = "INSERT INTO posting_assets (posting_id, asset_id) VALUES ($1, $2)";
            self.client.execute(insert_query, &[&posting.id, asset_id]).await?;
            log::debug!("Associated asset ID: {} with posting ID: {}", asset_id, posting.id);
        }
        
        log::info!("Successfully upserted posting with ID: {} and associated {} assets", posting.id, posting.asset_ids.len());
        Ok(())
    }

    pub async fn get_folder_contents(
        &self,
        folder_name: &str,
    ) -> Result<Option<Vec<Uuid>>, Box<dyn std::error::Error>> {
        log::debug!("Attempting to get contents for folder: {}", folder_name);

        let folder_query = "SELECT id FROM folders WHERE name = $1";
        let folder_rows = self.client.query(folder_query, &[&folder_name]).await?;
        
        if let Some(folder_row) = folder_rows.get(0) {
            let folder_id: Uuid = folder_row.get(0);
            log::debug!("Found folder with ID: {} for name: {}", folder_id, folder_name);
    
            let asset_query = "SELECT asset_id FROM asset_folders WHERE folder_id = $1";
            let asset_rows = self.client.query(asset_query, &[&folder_id]).await?;
            
            let mut asset_ids = Vec::new();
            for row in asset_rows {
                let asset_id: Uuid = row.get(0);
                asset_ids.push(asset_id);
            }
            
            log::info!("Retrieved {} assets from folder: {}", asset_ids.len(), folder_name);
            Ok(Some(asset_ids))
        } else {
            log::debug!("Folder not found: {}", folder_name);
            Ok(None)
        }
    }

    pub async fn insert_folder_contents(
        &self,
        folder_name: &str,
        contents: &Vec<Uuid>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting to insert folder contents for folder: {}, with {} assets", folder_name, contents.len());

        let folder_query = "INSERT INTO folders (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = $1 RETURNING id";
        let folder_row = self.client.query_one(folder_query, &[&folder_name]).await?;
        let folder_id: Uuid = folder_row.get(0);
        log::debug!("Got/created folder with ID: {} for name: {}", folder_id, folder_name);

        let delete_query = "DELETE FROM asset_folders WHERE folder_id = $1";
        let delete_result = self.client.execute(delete_query, &[&folder_id]).await?;
        log::debug!("Cleared {} existing asset associations for folder ID: {}", delete_result, folder_id);
        

        for asset_id in contents {
            let insert_query = "INSERT INTO asset_folders (folder_id, asset_id) VALUES ($1, $2)";
            self.client.execute(insert_query, &[&folder_id, asset_id]).await?;
            log::debug!("Associated asset ID: {} with folder ID: {}", asset_id, folder_id);
        }
        
        log::info!("Successfully updated folder contents for folder: {}, with {} assets", folder_name, contents.len());
        Ok(())
    }

    pub async fn get_posting_by_id_with_assets(
        &self,
        posting_id: &Uuid,
    ) -> Result<Option<crate::posting::models::Posting>, Box<dyn std::error::Error>> {
        log::debug!("Attempting to retrieve posting with ID: {} and its associated assets", posting_id);
        
        // Get the main posting data
        let posting_query = "SELECT * FROM postings WHERE id = $1";
        let posting_rows = self.client.query(posting_query, &[posting_id]).await?;
        
        if let Some(posting_row) = posting_rows.get(0) {
            // Convert the row to JSON and deserialize to Posting struct
            let mut posting_json = row_to_json(posting_row);
            
            // Get the associated asset IDs
            let asset_query = "SELECT asset_id FROM posting_assets WHERE posting_id = $1 ORDER BY asset_id";
            let asset_rows = self.client.query(asset_query, &[posting_id]).await?;
            
            let mut asset_ids = Vec::new();
            for row in asset_rows {
                let asset_id: Uuid = row.get(0);
                asset_ids.push(asset_id);
            }
            
            // Add asset_ids to the JSON object before deserializing
            if let serde_json::Value::Object(ref mut obj) = posting_json {
                obj.insert("asset_ids".to_string(), serde_json::Value::Array(
                    asset_ids.iter().map(|id| serde_json::Value::String(id.to_string())).collect()
                ));
            }
            
            let posting: crate::posting::models::Posting = serde_json::from_value(posting_json)?;
            log::debug!("Successfully retrieved posting with ID: {} and {} associated assets", posting.id, posting.asset_ids.len());
            Ok(Some(posting))
        } else {
            log::debug!("Posting with ID: {} not found", posting_id);
            Ok(None)
        }
    }

    pub async fn get_all_postings_with_assets(
        &self,
    ) -> Result<Vec<crate::posting::models::Posting>, Box<dyn std::error::Error>> {
        log::debug!("Attempting to retrieve all postings with their associated assets");
        
        // Get all postings
        let posting_query = "SELECT * FROM postings ORDER BY created_at DESC";
        let posting_rows = self.client.query(posting_query, &[]).await?;
        
        // Get all posting-asset relationships
        let asset_query = "SELECT posting_id, asset_id FROM posting_assets ORDER BY posting_id, asset_id";
        let asset_rows = self.client.query(asset_query, &[]).await?;
        
        // Create a map from posting_id to asset_ids
        let mut asset_map: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
        for row in asset_rows {
            let posting_id: Uuid = row.get(0);
            let asset_id: Uuid = row.get(1);
            
            asset_map.entry(posting_id).or_default().push(asset_id);
        }
        
        let mut postings = Vec::new();
        for row in posting_rows {
            let mut posting_json = row_to_json(&row);
            
            // Get the posting ID to look up associated assets
            let posting_id: Uuid = row.get("id");
            let asset_ids = asset_map.get(&posting_id).cloned().unwrap_or_default();
            
            // Add asset_ids to the JSON object before deserializing
            if let serde_json::Value::Object(ref mut obj) = posting_json {
                obj.insert("asset_ids".to_string(), serde_json::Value::Array(
                    asset_ids.iter().map(|id| serde_json::Value::String(id.to_string())).collect()
                ));
            }
            
            let posting: crate::posting::models::Posting = serde_json::from_value(posting_json)?;
            postings.push(posting);
        }
        
        log::info!("Successfully retrieved {} postings with their associated assets", postings.len());
        Ok(postings)
    }
}


fn row_to_json(row: &tokio_postgres::Row) -> Value {
    let mut map = serde_json::Map::new();
    
    for (idx, column) in row.columns().iter().enumerate() {

        let column_name = column.name();
        let value: Value = match column.type_().name() {
            "uuid" => {
                let uuid_value: Uuid = row.get(idx);
                Value::String(uuid_value.to_string())
            },
            "text" | "varchar" => {
                let opt_value: Option<String> = row.get(idx);
                match opt_value {
                    Some(s) => Value::String(s),
                    None => Value::Null,
                }
            },
            "int4" => {
                let opt_value: Option<i32> = row.get(idx);
                match opt_value {
                    Some(n) => Value::Number(n.into()),
                    None => Value::Null,
                }
            },
            "int8" => {
                let opt_value: Option<i64> = row.get(idx);
                match opt_value {
                    Some(n) => Value::Number(serde_json::Number::from(n)),
                    None => Value::Null,
                }
            },
            "bool" => {
                let opt_value: Option<bool> = row.get(idx);
                match opt_value {
                    Some(b) => Value::Bool(b),
                    None => Value::Null,
                }
            },
            "timestamptz" | "timestamp" => {
                let opt_value: Option<chrono::DateTime<chrono::Utc>> = row.get(idx);
                match opt_value {
                    Some(dt) => Value::String(dt.to_rfc3339()),
                    None => Value::Null,
                }
            },
            "date" => {
                let opt_value: Option<chrono::NaiveDate> = row.get(idx);
                match opt_value {
                    Some(d) => Value::String(d.to_string()),
                    None => Value::Null,
                }
            },
            _ => {

                let opt_str: Option<String> = row.get(idx);
                match opt_str {
                    Some(s) => Value::String(s),
                    None => Value::Null,
                }
            }
        };
        map.insert(column_name.to_string(), value);
    }
    
    Value::Object(map)
}