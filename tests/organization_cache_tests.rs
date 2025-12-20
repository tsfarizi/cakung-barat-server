//! Tests for organization cache behavior (Data → RAM).
//!
//! These tests verify:
//! 1. Cache correctly stores and retrieves organization data
//! 2. Cache updates are immediate
//! 3. Cache is used before storage

use cakung_barat_server::organization::model::OrganizationMember;
use moka::future::Cache;
use std::time::Duration;

const ORGANIZATION_CACHE_KEY: &str = "org_members";

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
async fn test_cache_stores_organization_members() {
    // Arrange
    let cache: Cache<String, Vec<OrganizationMember>> = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(10)
        .build();

    let members = vec![
        create_test_member(1, "User 1"),
        create_test_member(2, "User 2"),
    ];

    // Act
    cache
        .insert(ORGANIZATION_CACHE_KEY.to_string(), members.clone())
        .await;

    // Assert
    let cached = cache.get(ORGANIZATION_CACHE_KEY).await;
    assert!(cached.is_some());
    let cached_members = cached.unwrap();
    assert_eq!(cached_members.len(), 2);
    assert_eq!(cached_members[0].id, 1);
    assert_eq!(cached_members[1].id, 2);
}

#[tokio::test]
async fn test_cache_update_replaces_previous_value() {
    // Arrange
    let cache: Cache<String, Vec<OrganizationMember>> = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(10)
        .build();

    // Insert initial data
    let initial = vec![create_test_member(1, "Initial User")];
    cache
        .insert(ORGANIZATION_CACHE_KEY.to_string(), initial)
        .await;

    // Act - Update with new data
    let updated = vec![
        create_test_member(1, "Updated User 1"),
        create_test_member(2, "New User 2"),
    ];
    cache
        .insert(ORGANIZATION_CACHE_KEY.to_string(), updated)
        .await;

    // Assert
    let cached = cache.get(ORGANIZATION_CACHE_KEY).await.unwrap();
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[0].name, Some("Updated User 1".to_string()));
    assert_eq!(cached[1].name, Some("New User 2".to_string()));
}

#[tokio::test]
async fn test_cache_returns_none_when_empty() {
    // Arrange
    let cache: Cache<String, Vec<OrganizationMember>> = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(10)
        .build();

    // Act & Assert
    let cached = cache.get(ORGANIZATION_CACHE_KEY).await;
    assert!(
        cached.is_none(),
        "Cache should return None when key doesn't exist"
    );
}

#[tokio::test]
async fn test_cache_invalidation_removes_data() {
    // Arrange
    let cache: Cache<String, Vec<OrganizationMember>> = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(10)
        .build();

    let members = vec![create_test_member(1, "User to Remove")];
    cache
        .insert(ORGANIZATION_CACHE_KEY.to_string(), members)
        .await;

    // Verify data exists
    assert!(cache.get(ORGANIZATION_CACHE_KEY).await.is_some());

    // Act - Invalidate
    cache.invalidate(ORGANIZATION_CACHE_KEY).await;

    // Assert
    assert!(cache.get(ORGANIZATION_CACHE_KEY).await.is_none());
}

#[tokio::test]
async fn test_cache_is_immediately_available_after_insert() {
    // Arrange
    let cache: Cache<String, Vec<OrganizationMember>> = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(10)
        .build();

    // Act
    let members = vec![create_test_member(1, "Immediate User")];
    let start = std::time::Instant::now();
    cache
        .insert(ORGANIZATION_CACHE_KEY.to_string(), members)
        .await;
    let cached = cache.get(ORGANIZATION_CACHE_KEY).await;
    let elapsed = start.elapsed();

    // Assert - Should be very fast (< 10ms)
    assert!(cached.is_some());
    assert!(
        elapsed.as_millis() < 10,
        "Cache insert and read should be immediate, took {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_cache_preserves_all_member_fields() {
    // Arrange
    let cache: Cache<String, Vec<OrganizationMember>> = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(10)
        .build();

    let member = OrganizationMember {
        id: 42,
        name: Some("Full Field Test".to_string()),
        position: "Manager".to_string(),
        photo: Some("manager.jpg".to_string()),
        parent_id: Some(1),
        level: 3,
        role: "kepala_seksi".to_string(),
    };

    // Act
    cache
        .insert(ORGANIZATION_CACHE_KEY.to_string(), vec![member])
        .await;

    // Assert
    let cached = cache.get(ORGANIZATION_CACHE_KEY).await.unwrap();
    assert_eq!(cached.len(), 1);
    let cached_member = &cached[0];
    assert_eq!(cached_member.id, 42);
    assert_eq!(cached_member.name, Some("Full Field Test".to_string()));
    assert_eq!(cached_member.position, "Manager");
    assert_eq!(cached_member.photo, Some("manager.jpg".to_string()));
    assert_eq!(cached_member.parent_id, Some(1));
    assert_eq!(cached_member.level, 3);
    assert_eq!(cached_member.role, "kepala_seksi");
}

#[tokio::test]
async fn test_write_through_pattern_cache_first() {
    // This test simulates the write-through pattern:
    // 1. Update cache immediately
    // 2. Data is available right away (without waiting for storage)

    let cache: Cache<String, Vec<OrganizationMember>> = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(10)
        .build();

    // Simulate write-through: update cache first
    let members = vec![create_test_member(1, "Write-Through User")];

    // Cache update (this is what happens in write_organization_data)
    cache
        .insert(ORGANIZATION_CACHE_KEY.to_string(), members.clone())
        .await;

    // Immediately read - should get data (simulating fast response to client)
    let cached = cache.get(ORGANIZATION_CACHE_KEY).await;
    assert!(
        cached.is_some(),
        "Data should be immediately available from cache"
    );
    assert_eq!(cached.unwrap()[0].id, 1);

    // In real implementation, background worker would persist to storage here
    // But client already got response!
}

#[tokio::test]
async fn test_cache_clone_does_not_affect_original() {
    // Verify that getting data from cache returns a copy,
    // and modifying it doesn't affect cached data

    let cache: Cache<String, Vec<OrganizationMember>> = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(10)
        .build();

    let members = vec![create_test_member(1, "Original")];
    cache
        .insert(ORGANIZATION_CACHE_KEY.to_string(), members)
        .await;

    // Get and modify
    let mut local_copy = cache.get(ORGANIZATION_CACHE_KEY).await.unwrap();
    local_copy.push(create_test_member(2, "New Local"));

    // Original cache should be unchanged
    let original = cache.get(ORGANIZATION_CACHE_KEY).await.unwrap();
    assert_eq!(original.len(), 1, "Original cache should not be modified");
}
