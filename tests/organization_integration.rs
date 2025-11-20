use actix_web::{test, web, App};
use cakung_barat_server::organization::model::{CreateMemberRequest, UpdateMemberRequest};
use cakung_barat_server::organization::routes;
use cakung_barat_server::storage::{ObjectStorage, SupabaseConfig, SupabaseStorage};
use cakung_barat_server::AppState;
use std::sync::Arc;

#[cfg(test)]
mod organization_integration_tests {
    use super::*;

    async fn create_test_app_state() -> web::Data<AppState> {
        dotenvy::dotenv().ok();
        
        let supabase_config = SupabaseConfig::from_env()
            .expect("Failed to load Supabase config from environment");
        
        let http_client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(900))
            .user_agent("cakung-barat-server-test/1.0")
            .build()
            .expect("Failed to create HTTP client");
        
        let storage: Arc<dyn ObjectStorage + Send + Sync> = 
            Arc::new(SupabaseStorage::new(supabase_config, http_client.clone()));
        
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&std::env::var("SUPABASE_DATABASE_URL").expect("SUPABASE_DATABASE_URL must be set"))
            .await
            .expect("Failed to create database pool");
        
        let state = AppState::new_with_pool_and_storage(pool, storage)
            .await
            .expect("Failed to create AppState");
        
        web::Data::new(state)
    }

    #[actix_web::test]
    async fn test_get_all_members() {
        let app_state = create_test_app_state().await;
        
        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .configure(routes::config)
        ).await;
        
        let req = test::TestRequest::get()
            .uri("/organization")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_create_and_delete_member() {
        let app_state = create_test_app_state().await;
        
        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .configure(routes::config)
        ).await;
        
        // Create a new member
        let create_req = CreateMemberRequest {
            name: "Test Member".to_string(),
            position: "Test Position".to_string(),
            photo: "test.jpg".to_string(),
            parent_id: None,
            x: 100,
            y: 200,
            role: "staf".to_string(),
        };
        
        let req = test::TestRequest::post()
            .uri("/organization")
            .set_json(&create_req)
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let created_member: cakung_barat_server::organization::model::OrganizationMember = 
            test::read_body_json(resp).await;
        
        assert_eq!(created_member.name, Some("Test Member".to_string()));
        assert_eq!(created_member.position, "Test Position");
        
        // Delete the member
        let delete_req = test::TestRequest::delete()
            .uri(&format!("/organization/{}", created_member.id))
            .to_request();
        
        let resp = test::call_service(&app, delete_req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_update_member() {
        let app_state = create_test_app_state().await;
        
        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .configure(routes::config)
        ).await;
        
        // Create a member first
        let create_req = CreateMemberRequest {
            name: "Original Name".to_string(),
            position: "Original Position".to_string(),
            photo: "original.jpg".to_string(),
            parent_id: None,
            x: 50,
            y: 75,
            role: "kasi".to_string(),
        };
        
        let req = test::TestRequest::post()
            .uri("/organization")
            .set_json(&create_req)
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        let created_member: cakung_barat_server::organization::model::OrganizationMember = 
            test::read_body_json(resp).await;
        
        // Update the member
        let update_req = UpdateMemberRequest {
            name: Some("Updated Name".to_string()),
            position: Some("Updated Position".to_string()),
            photo: None,
            parent_id: None,
            x: Some(150),
            y: Some(250),
            role: None,
        };
        
        let req = test::TestRequest::put()
            .uri(&format!("/organization/{}", created_member.id))
            .set_json(&update_req)
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let updated_member: cakung_barat_server::organization::model::OrganizationMember = 
            test::read_body_json(resp).await;
        
        assert_eq!(updated_member.name, Some("Updated Name".to_string()));
        assert_eq!(updated_member.position, "Updated Position");
        assert_eq!(updated_member.x, 150);
        assert_eq!(updated_member.y, 250);
        
        // Clean up
        let delete_req = test::TestRequest::delete()
            .uri(&format!("/organization/{}", created_member.id))
            .to_request();
        
        test::call_service(&app, delete_req).await;
    }

    #[actix_web::test]
    async fn test_cache_behavior() {
        let app_state = create_test_app_state().await;
        
        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .configure(routes::config)
        ).await;
        
        // First request should be a cache miss
        let req1 = test::TestRequest::get()
            .uri("/organization")
            .to_request();
        
        let resp1 = test::call_service(&app, req1).await;
        assert!(resp1.status().is_success());
        
        // Second request should be a cache hit
        let req2 = test::TestRequest::get()
            .uri("/organization")
            .to_request();
        
        let resp2 = test::call_service(&app, req2).await;
        assert!(resp2.status().is_success());
        
        // Create a new member to invalidate cache
        let create_req = CreateMemberRequest {
            name: "Cache Test".to_string(),
            position: "Test".to_string(),
            photo: "cache.jpg".to_string(),
            parent_id: None,
            x: 0,
            y: 0,
            role: "staf".to_string(),
        };
        
        let req = test::TestRequest::post()
            .uri("/organization")
            .set_json(&create_req)
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        let created_member: cakung_barat_server::organization::model::OrganizationMember = 
            test::read_body_json(resp).await;
        
        // Next request should be a cache miss again (cache was invalidated)
        let req3 = test::TestRequest::get()
            .uri("/organization")
            .to_request();
        
        let resp3 = test::call_service(&app, req3).await;
        assert!(resp3.status().is_success());
        
        // Clean up
        let delete_req = test::TestRequest::delete()
            .uri(&format!("/organization/{}", created_member.id))
            .to_request();
        
        test::call_service(&app, delete_req).await;
    }
}
