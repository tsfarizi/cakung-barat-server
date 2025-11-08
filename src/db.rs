use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::schema::Uuid;
use crate::posting::models::{Posting};
use crate::asset::models::{Asset};

pub struct AppState {
    pub postings: RwLock<HashMap<Uuid, Posting>>,
    pub assets: RwLock<HashMap<Uuid, Asset>>,
    pub folders: RwLock<HashMap<String, Vec<Uuid>>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            postings: RwLock::new(HashMap::new()),
            assets: RwLock::new(HashMap::new()),
            folders: RwLock::new(HashMap::new()),
        }
    }
}

pub type SharedAppState = Arc<AppState>;
