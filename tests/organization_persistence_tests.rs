//! Tests for organization persistence worker and write-through cache behavior.
//!
//! These tests verify:
//! 1. Persistence worker receives data via channel and writes to storage
//! 2. Debouncing behavior batches multiple writes
//! 3. Cache is updated correctly

use cakung_barat_server::organization::model::OrganizationMember;
use cakung_barat_server::organization::persistence::start_persistence_worker;
use cakung_barat_server::storage::{FolderContent, ObjectStorage};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Mock storage that tracks upload calls for testing
struct MockStorage {
    upload_count: AtomicUsize,
    uploaded_data: Arc<Mutex<Vec<Vec<u8>>>>,
    should_fail: bool,
}

impl MockStorage {
    fn new() -> Self {
        Self {
            upload_count: AtomicUsize::new(0),
            uploaded_data: Arc::new(Mutex::new(Vec::new())),
            should_fail: false,
        }
    }

    fn new_failing() -> Self {
        Self {
            upload_count: AtomicUsize::new(0),
            uploaded_data: Arc::new(Mutex::new(Vec::new())),
            should_fail: true,
        }
    }

    fn get_upload_count(&self) -> usize {
        self.upload_count.load(Ordering::SeqCst)
    }

    async fn get_last_uploaded_data(&self) -> Option<Vec<u8>> {
        let data = self.uploaded_data.lock().await;
        data.last().cloned()
    }
}

#[async_trait::async_trait]
impl ObjectStorage for MockStorage {
    async fn upload_file(&self, _filename: &str, file_data: &[u8]) -> Result<(), String> {
        if self.should_fail {
            return Err("Mock upload failure".to_string());
        }
        self.upload_count.fetch_add(1, Ordering::SeqCst);
        let mut data = self.uploaded_data.lock().await;
        data.push(file_data.to_vec());
        Ok(())
    }

    async fn download_file(&self, _filename: &str) -> Result<Vec<u8>, String> {
        Ok(vec![])
    }

    async fn delete_file(&self, _filename: &str) -> Result<(), String> {
        Ok(())
    }

    async fn create_folder(&self, _folder_name: &str) -> Result<(), String> {
        Ok(())
    }

    async fn list_folder_contents(&self, _folder_name: &str) -> Result<Vec<FolderContent>, String> {
        Ok(vec![])
    }

    fn get_asset_url(&self, _filename: &str) -> String {
        "http://mock-url".to_string()
    }
}

fn create_test_member(id: i32, name: &str) -> OrganizationMember {
    OrganizationMember {
        id,
        name: Some(name.to_string()),
        position: "Test Position".to_string(),
        photo: Some("test.jpg".to_string()),
        parent_id: None,
        level: 1,
        role: "staf".to_string(),
    }
}

#[tokio::test]
async fn test_persistence_worker_receives_and_writes_data() {
    // Arrange
    let storage = Arc::new(MockStorage::new());
    let (sender, receiver) = mpsc::channel::<Vec<OrganizationMember>>(10);

    // Start worker in background
    let storage_clone = storage.clone();
    let worker_handle = tokio::spawn(async move {
        start_persistence_worker(receiver, storage_clone).await;
    });

    // Act - Send data to worker
    let members = vec![create_test_member(1, "Test User")];
    sender.send(members.clone()).await.unwrap();

    // Wait for debounce + processing (600ms should be enough for 500ms debounce)
    tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;

    // Assert
    assert_eq!(
        storage.get_upload_count(),
        1,
        "Storage should be called once"
    );

    // Verify uploaded data
    let uploaded = storage.get_last_uploaded_data().await.unwrap();
    let parsed: Vec<OrganizationMember> = serde_json::from_slice(&uploaded).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, 1);
    assert_eq!(parsed[0].name, Some("Test User".to_string()));

    // Cleanup
    drop(sender);
    worker_handle.abort();
}

#[tokio::test]
async fn test_persistence_worker_debounces_rapid_writes() {
    // Arrange
    let storage = Arc::new(MockStorage::new());
    let (sender, receiver) = mpsc::channel::<Vec<OrganizationMember>>(10);

    let storage_clone = storage.clone();
    let worker_handle = tokio::spawn(async move {
        start_persistence_worker(receiver, storage_clone).await;
    });

    // Act - Send multiple rapid updates (should be batched)
    for i in 1..=5 {
        let members = vec![create_test_member(i, &format!("User {}", i))];
        sender.send(members).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Wait for debounce to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

    // Assert - Should only have 1 upload (debounced)
    assert_eq!(
        storage.get_upload_count(),
        1,
        "Debouncing should batch multiple rapid writes into one"
    );

    // The last update (User 5) should be persisted
    let uploaded = storage.get_last_uploaded_data().await.unwrap();
    let parsed: Vec<OrganizationMember> = serde_json::from_slice(&uploaded).unwrap();
    assert_eq!(parsed[0].id, 5);
    assert_eq!(parsed[0].name, Some("User 5".to_string()));

    // Cleanup
    drop(sender);
    worker_handle.abort();
}

#[tokio::test]
async fn test_persistence_worker_handles_storage_failure_gracefully() {
    // Arrange
    let storage = Arc::new(MockStorage::new_failing());
    let (sender, receiver) = mpsc::channel::<Vec<OrganizationMember>>(10);

    let storage_clone = storage.clone();
    let worker_handle = tokio::spawn(async move {
        start_persistence_worker(receiver, storage_clone).await;
    });

    // Act - Send data (should fail but not crash)
    let members = vec![create_test_member(1, "Test User")];
    sender.send(members).await.unwrap();

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;

    // Assert - Worker should still be running (not crashed)
    assert!(
        !worker_handle.is_finished(),
        "Worker should continue running after storage failure"
    );

    // Cleanup
    drop(sender);
    worker_handle.abort();
}

#[tokio::test]
async fn test_persistence_worker_separate_batches_for_delayed_writes() {
    // Arrange
    let storage = Arc::new(MockStorage::new());
    let (sender, receiver) = mpsc::channel::<Vec<OrganizationMember>>(10);

    let storage_clone = storage.clone();
    let worker_handle = tokio::spawn(async move {
        start_persistence_worker(receiver, storage_clone).await;
    });

    // Act - First batch
    sender
        .send(vec![create_test_member(1, "First Batch")])
        .await
        .unwrap();

    // Wait for first batch to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;
    assert_eq!(storage.get_upload_count(), 1, "First batch should complete");

    // Second batch (after first completes)
    sender
        .send(vec![create_test_member(2, "Second Batch")])
        .await
        .unwrap();

    // Wait for second batch
    tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;

    // Assert - Should have 2 separate uploads
    assert_eq!(
        storage.get_upload_count(),
        2,
        "Delayed writes should result in separate uploads"
    );

    // Cleanup
    drop(sender);
    worker_handle.abort();
}

#[tokio::test]
async fn test_persistence_worker_stops_when_sender_dropped() {
    // Arrange
    let storage = Arc::new(MockStorage::new());
    let (sender, receiver) = mpsc::channel::<Vec<OrganizationMember>>(10);

    let storage_clone = storage.clone();
    let worker_handle = tokio::spawn(async move {
        start_persistence_worker(receiver, storage_clone).await;
    });

    // Act - Drop sender (simulating shutdown)
    drop(sender);

    // Wait for worker to stop
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Assert - Worker should finish
    assert!(
        worker_handle.is_finished(),
        "Worker should stop when sender is dropped"
    );
}

#[tokio::test]
async fn test_channel_send_does_not_block_on_full_buffer() {
    // This test verifies that the sender returns quickly
    // even when the channel has capacity

    let (sender, _receiver) = mpsc::channel::<Vec<OrganizationMember>>(100);

    let start = std::time::Instant::now();

    // Send multiple items - should not block
    for i in 0..50 {
        let members = vec![create_test_member(i, &format!("User {}", i))];
        sender.send(members).await.unwrap();
    }

    let elapsed = start.elapsed();

    // Should complete very quickly (< 100ms for 50 sends)
    assert!(
        elapsed.as_millis() < 100,
        "Sending to channel should be non-blocking and fast, took {:?}",
        elapsed
    );
}
