use dotenvy::dotenv;
use serde::{Serialize, de::DeserializeOwned};
use std::env;
use std::sync::Arc;
use tokio_postgres::NoTls;
use uuid::Uuid;
use serde_json::Value;

pub struct AppState {
    pub client: Arc<tokio_postgres::Client>,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();
        
        let database_url = env::var("SUPABASE_DATABASE_URL")
            .unwrap_or_else(|_| panic!("SUPABASE_DATABASE_URL must be set"));

        let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
        
        // Spawn the connection to run in the background
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
        let query = format!("SELECT * FROM {} WHERE id = $1", table_name);
        let rows = self.client.query(&query, &[key]).await?;
        
        if let Some(row) = rows.get(0) {
            // Convert the row to JSON and then deserialize to the target type
            let json_value = row_to_json(row);
            let item: T = serde_json::from_value(json_value)?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all_items<T: DeserializeOwned>(
        &self,
        table_name: &str,
    ) -> Result<Vec<T>, Box<dyn std::error::Error>> {
        let query = format!("SELECT * FROM {}", table_name);
        let rows = self.client.query(&query, &[]).await?;
        
        let mut items = Vec::new();
        for row in rows {
            let json_value = row_to_json(&row);
            let item: T = serde_json::from_value(json_value)?;
            items.push(item);
        }
        
        Ok(items)
    }

    pub async fn insert_item<T: Serialize>(
        &self,
        table_name: &str,
        _key: &Uuid,  // Note: This parameter is not used, but kept for API compatibility
        item: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_value = serde_json::to_value(item)?;
        
        // Create dynamic INSERT query based on the table
        let query = match table_name {
            "assets" => {
                "INSERT INTO assets (id, name, filename, url, description, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, NOW(), NOW()) ON CONFLICT (id) DO UPDATE SET name = $2, filename = $3, url = $4, description = $5, updated_at = NOW()"
            },
            "postings" => {
                "INSERT INTO postings (id, judul, tanggal, detail, created_at, updated_at) VALUES ($1, $2, $3, $4, NOW(), NOW()) ON CONFLICT (id) DO UPDATE SET judul = $2, tanggal = $3, detail = $4, updated_at = NOW()"
            },
            _ => {
                return Err("Unsupported table for insert_item".into());
            }
        };
        
        match table_name {
            "assets" => {
                let asset: crate::asset::models::Asset = serde_json::from_value(json_value)?;
                self.client.execute(
                    query,
                    &[&asset.id, &asset.name, &asset.filename, &asset.url, &asset.description.as_ref().map(|s| s.as_str())],
                ).await?;
            },
            "postings" => {
                let posting: crate::posting::models::Posting = serde_json::from_value(json_value)?;
                self.client.execute(
                    query,
                    &[&posting.id, &posting.judul, &posting.tanggal, &posting.detail],
                ).await?;
            },
            _ => {
                return Err("Unsupported table for insert_item".into());
            }
        };
        
        Ok(())
    }

    pub async fn delete_item(
        &self,
        table_name: &str,
        key: &Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let query = format!("DELETE FROM {} WHERE id = $1", table_name);
        self.client.execute(&query, &[key]).await?;
        Ok(())
    }

    pub async fn get_folder_contents(
        &self,
        folder_name: &str,
    ) -> Result<Option<Vec<Uuid>>, Box<dyn std::error::Error>> {
        // Get folder ID first
        let folder_query = "SELECT id FROM folders WHERE name = $1";
        let folder_rows = self.client.query(folder_query, &[&folder_name]).await?;
        
        if let Some(folder_row) = folder_rows.get(0) {
            let folder_id: Uuid = folder_row.get(0);
            
            // Get all asset IDs associated with this folder
            let asset_query = "SELECT asset_id FROM asset_folders WHERE folder_id = $1";
            let asset_rows = self.client.query(asset_query, &[&folder_id]).await?;
            
            let mut asset_ids = Vec::new();
            for row in asset_rows {
                let asset_id: Uuid = row.get(0);
                asset_ids.push(asset_id);
            }
            
            Ok(Some(asset_ids))
        } else {
            Ok(None) // Folder doesn't exist
        }
    }

    pub async fn insert_folder_contents(
        &self,
        folder_name: &str,
        contents: &Vec<Uuid>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get or create the folder
        let folder_query = "INSERT INTO folders (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = $1 RETURNING id";
        let folder_row = self.client.query_one(folder_query, &[&folder_name]).await?;
        let folder_id: Uuid = folder_row.get(0);
        
        // Clear existing associations for this folder
        let delete_query = "DELETE FROM asset_folders WHERE folder_id = $1";
        self.client.execute(delete_query, &[&folder_id]).await?;
        
        // Insert new associations
        for asset_id in contents {
            let insert_query = "INSERT INTO asset_folders (folder_id, asset_id) VALUES ($1, $2)";
            self.client.execute(insert_query, &[&folder_id, asset_id]).await?;
        }
        
        Ok(())
    }
}

// Helper function to convert a row to JSON
fn row_to_json(row: &tokio_postgres::Row) -> Value {
    let mut map = serde_json::Map::new();
    
    for (idx, column) in row.columns().iter().enumerate() {
        // Handle each column type individually
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
                // For other types, try to get as a generic value
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