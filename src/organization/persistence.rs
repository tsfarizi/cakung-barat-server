//! Background persistence worker for organization data.
//!
//! This module provides an async worker that persists organization data to Supabase Storage
//! with debouncing to batch multiple writes.

use crate::organization::model::OrganizationMember;
use crate::storage::ObjectStorage;
use std::sync::Arc;
use tokio::sync::mpsc;

const ORGANIZATION_FILE: &str = "organization.json";
const DEBOUNCE_MS: u64 = 500;

/// Starts the background persistence worker.
///
/// The worker receives organization data via channel and persists it to storage.
/// It uses debouncing to batch multiple writes within a short time window.
pub async fn start_persistence_worker(
    mut receiver: mpsc::Receiver<Vec<OrganizationMember>>,
    storage: Arc<dyn ObjectStorage + Send + Sync>,
) {
    log::info!("Organization persistence worker started");

    while let Some(members) = receiver.recv().await {
        // Debounce: drain any pending messages to get the latest
        let mut latest = members;
        while let Ok(newer) = receiver.try_recv() {
            log::debug!("Batching pending organization update");
            latest = newer;
        }

        // Small delay to allow more batching if writes come in rapid succession
        tokio::time::sleep(tokio::time::Duration::from_millis(DEBOUNCE_MS)).await;

        // Drain again after delay to capture any writes during the wait
        while let Ok(newer) = receiver.try_recv() {
            log::debug!("Batching organization update after debounce delay");
            latest = newer;
        }

        // Persist to storage
        match serde_json::to_vec(&latest) {
            Ok(json_data) => {
                if let Err(e) = storage.upload_file(ORGANIZATION_FILE, &json_data).await {
                    log::error!("Failed to persist organization data to storage: {}", e);
                } else {
                    log::info!(
                        "Organization data persisted to storage ({} members)",
                        latest.len()
                    );
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to serialize organization data for persistence: {}",
                    e
                );
            }
        }
    }

    log::info!("Organization persistence worker stopped");
}
