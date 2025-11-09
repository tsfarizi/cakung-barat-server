use std::sync::Arc;
use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use uuid::Uuid;
use serde::{Serialize, de::DeserializeOwned};
use dotenvy::dotenv;
use std::env;

pub struct AppState {
    pub db: Arc<DB>,
}

impl AppState {
    pub fn new() -> Self {
        dotenv().ok();
        let path = env::var("DATABASE_URL").unwrap_or_else(|_| {
            eprintln!("DATABASE_URL not set in .env or environment, using default path: /data/database");
            String::from("/data/database")
        });
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        let postings_cf = ColumnFamilyDescriptor::new("postings", Options::default());
        let assets_cf = ColumnFamilyDescriptor::new("assets", Options::default());
        let folders_cf = ColumnFamilyDescriptor::new("folders", Options::default());

        let db = DB::open_cf_descriptors(&db_opts, path, vec![postings_cf, assets_cf, folders_cf]).unwrap();

        AppState {
            db: Arc::new(db),
        }
    }

    pub fn get_item<T: DeserializeOwned>(&self, cf_name: &str, key: &Uuid) -> Result<Option<T>, rocksdb::Error> {
        let cf = self.db.cf_handle(cf_name).unwrap();
        let key_bytes = key.to_string();
        match self.db.get_cf(&cf, key_bytes.as_bytes())? {
            Some(value) => {
                let item: T = serde_json::from_slice(&value).unwrap();
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }

    pub fn get_all_items<T: DeserializeOwned>(&self, cf_name: &str) -> Result<Vec<T>, rocksdb::Error> {
        let cf = self.db.cf_handle(cf_name).unwrap();
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        let mut items = Vec::new();
        for item in iter {
            let (_key, value) = item.unwrap();
            let item: T = serde_json::from_slice(&value).unwrap();
            items.push(item);
        }
        Ok(items)
    }

    pub fn insert_item<T: Serialize>(&self, cf_name: &str, key: &Uuid, item: &T) -> Result<(), rocksdb::Error> {
        let cf = self.db.cf_handle(cf_name).unwrap();
        let key_bytes = key.to_string();
        let value = serde_json::to_vec(item).unwrap();
        self.db.put_cf(&cf, key_bytes.as_bytes(), value)
    }

    pub fn delete_item(&self, cf_name: &str, key: &Uuid) -> Result<(), rocksdb::Error> {
        let cf = self.db.cf_handle(cf_name).unwrap();
        let key_bytes = key.to_string();
        self.db.delete_cf(&cf, key_bytes.as_bytes())
    }

    pub fn get_folder_contents(&self, folder_name: &str) -> Result<Option<Vec<Uuid>>, rocksdb::Error> {
        let cf = self.db.cf_handle("folders").unwrap();
        match self.db.get_cf(&cf, folder_name.as_bytes())? {
            Some(value) => {
                let item: Vec<Uuid> = serde_json::from_slice(&value).unwrap();
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }

    pub fn insert_folder_contents(&self, folder_name: &str, contents: &Vec<Uuid>) -> Result<(), rocksdb::Error> {
        let cf = self.db.cf_handle("folders").unwrap();
        let value = serde_json::to_vec(contents).unwrap();
        self.db.put_cf(&cf, folder_name.as_bytes(), value)
    }
}
