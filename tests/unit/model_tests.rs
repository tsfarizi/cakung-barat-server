#[cfg(test)]
mod asset_model_tests {
    use crate::asset::models::Asset;

    #[test]
    fn test_asset_new_with_description() {
        let name = "Test Asset".to_string();
        let filename = "test_file.jpg".to_string();
        let url = "/assets/serve/test_file.jpg".to_string();
        let description = Some("A test asset".to_string());

        let asset = Asset::new(name.clone(), filename.clone(), url.clone(), description.clone());

        // Check that the asset was created with the correct values
        assert_eq!(asset.name, name);
        assert_eq!(asset.filename, filename);
        assert_eq!(asset.url, url);
        assert_eq!(asset.description, description);

        // Check that the ID is not nil (ensuring Uuid::new_v4() worked)
        assert!(!asset.id.is_nil());

        // Check that timestamps are set
        assert!(asset.created_at.is_some());
        assert!(asset.updated_at.is_some());
    }

    #[test]
    fn test_asset_new_without_description() {
        let name = "Test Asset".to_string();
        let filename = "test_file.jpg".to_string();
        let url = "/assets/serve/test_file.jpg".to_string();
        let description = None;

        let asset = Asset::new(name.clone(), filename.clone(), url.clone(), description);

        assert_eq!(asset.name, name);
        assert_eq!(asset.filename, filename);
        assert_eq!(asset.url, url);
        assert_eq!(asset.description, None);

        assert!(!asset.id.is_nil());
        assert!(asset.created_at.is_some());
        assert!(asset.updated_at.is_some());
    }
}

#[cfg(test)]
mod post_model_tests {
    use crate::posting::models::Post;
    use uuid::Uuid;
    use chrono::{NaiveDate, Utc};

    #[test]
    fn test_post_new() {
        let title = "Test Title".to_string();
        let category = "Test Category".to_string();
        let excerpt = "Test excerpt".to_string();
        let img = Some(vec![Uuid::new_v4()]);

        let post = Post::new(title.clone(), category.clone(), excerpt.clone(), img.clone());

        // Check that the post was created with the correct values
        assert_eq!(post.title, title);
        assert_eq!(post.category, category);
        assert_eq!(post.excerpt, excerpt);
        assert_eq!(post.img, img);

        // Check that the ID is not nil (ensuring Uuid::new_v4() worked)
        assert!(!post.id.is_nil());

        // Check that dates and timestamps are set
        assert!(post.created_at.is_some());
        assert!(post.updated_at.is_some());
    }

    #[test]
    fn test_post_new_without_images() {
        let title = "Test Title".to_string();
        let category = "Test Category".to_string();
        let excerpt = "Test excerpt".to_string();
        let img = None;

        let post = Post::new(title.clone(), category.clone(), excerpt.clone(), img);

        assert_eq!(post.title, title);
        assert_eq!(post.category, category);
        assert_eq!(post.excerpt, excerpt);
        assert_eq!(post.img, None);

        assert!(!post.id.is_nil());
        assert!(post.created_at.is_some());
        assert!(post.updated_at.is_some());
    }
    
    #[test]
    fn test_post_struct_fields() {
        let post = Post {
            id: Uuid::new_v4(),
            title: "Sample Title".to_string(),
            category: "Sample Category".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 11, 12).unwrap(),
            excerpt: "Sample excerpt".to_string(),
            img: Some(vec![Uuid::new_v4(), Uuid::new_v4()]),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        };
        
        assert_eq!(post.title, "Sample Title");
        assert_eq!(post.category, "Sample Category");
        assert_eq!(post.excerpt, "Sample excerpt");
        assert!(post.img.is_some());
        assert_eq!(post.img.as_ref().unwrap().len(), 2);
        assert!(post.created_at.is_some());
        assert!(post.updated_at.is_some());
    }
}

#[cfg(test)]
mod posting_model_tests {
    use crate::posting::models::{Posting, Post};
    use uuid::Uuid;
    use chrono::{NaiveDate, Utc};

    #[test]
    fn test_posting_struct() {
        let posting = Posting {
            id: Uuid::new_v4(),
            title: "Test Posting".to_string(),
            category: "Test Category".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 11, 12).unwrap(),
            excerpt: "Test excerpt".to_string(),
            img: Some(vec![Uuid::new_v4()]),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            asset_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };
        
        assert_eq!(posting.title, "Test Posting");
        assert_eq!(posting.category, "Test Category");
        assert_eq!(posting.excerpt, "Test excerpt");
        assert!(posting.img.is_some());
        assert_eq!(posting.asset_ids.len(), 2);
        assert!(posting.created_at.is_some());
        assert!(posting.updated_at.is_some());
    }

    #[test]
    fn test_posting_to_post_conversion_concept() {
        // This test verifies that Posting and Post models have compatible fields
        let post = Post {
            id: Uuid::new_v4(),
            title: "Test Post".to_string(),
            category: "Test Category".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 11, 12).unwrap(),
            excerpt: "Test excerpt".to_string(),
            img: Some(vec![Uuid::new_v4()]),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        };
        
        let posting = Posting {
            id: post.id,
            title: post.title.clone(),
            category: post.category.clone(),
            date: post.date,
            excerpt: post.excerpt.clone(),
            img: post.img.clone(),
            created_at: post.created_at,
            updated_at: post.updated_at,
            asset_ids: vec![Uuid::new_v4()],
        };
        
        assert_eq!(posting.title, post.title);
        assert_eq!(posting.category, post.category);
        assert_eq!(posting.excerpt, post.excerpt);
        assert_eq!(posting.img, post.img);
    }
}

#[cfg(test)]
mod request_model_tests {
    use crate::posting::models::{CreatePostingRequest, UpdatePostingRequest};
    use uuid::Uuid;

    #[test]
    fn test_create_posting_request() {
        let create_req = CreatePostingRequest {
            title: "New Post Title".to_string(),
            category: "New Category".to_string(),
            excerpt: "New excerpt".to_string(),
            img: Some(vec![Uuid::new_v4()]),
        };
        
        assert_eq!(create_req.title, "New Post Title");
        assert_eq!(create_req.category, "New Category");
        assert_eq!(create_req.excerpt, "New excerpt");
        assert!(create_req.img.is_some());
        assert_eq!(create_req.img.as_ref().unwrap().len(), 1);
    }
    
    #[test]
    fn test_update_posting_request() {
        let update_req = UpdatePostingRequest {
            title: Some("Updated Title".to_string()),
            category: Some("Updated Category".to_string()),
            excerpt: Some("Updated excerpt".to_string()),
            img: Some(vec![Uuid::new_v4(), Uuid::new_v4()]),
        };
        
        assert_eq!(update_req.title, Some("Updated Title".to_string()));
        assert_eq!(update_req.category, Some("Updated Category".to_string()));
        assert_eq!(update_req.excerpt, Some("Updated excerpt".to_string()));
        assert!(update_req.img.is_some());
        assert_eq!(update_req.img.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_update_posting_request_partial() {
        let update_req = UpdatePostingRequest {
            title: None, // Not updating title
            category: Some("Updated Category".to_string()),
            excerpt: None, // Not updating excerpt
            img: None, // Not updating images
        };
        
        assert_eq!(update_req.title, None);
        assert_eq!(update_req.category, Some("Updated Category".to_string()));
        assert_eq!(update_req.excerpt, None);
        assert!(update_req.img.is_none());
    }
}