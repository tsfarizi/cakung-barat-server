use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;
use crate::models::{Posting};

pub struct AppState {
    pub postings: RwLock<HashMap<Uuid, Posting>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            postings: RwLock::new(HashMap::new()),
        }
    }
}

pub type SharedAppState = Arc<AppState>;
