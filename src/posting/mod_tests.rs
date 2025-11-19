#[cfg(test)]
mod tests {
    use crate::posting::models::Post;

    #[test]
    fn test_post_new() {
        let title = "Test Title".to_string();
        let category = "Test Category".to_string();
        let excerpt = "Test excerpt".to_string();
        let folder_id = Some("posts/some-folder-id".to_string());

        let post = Post::new(title.clone(), category.clone(), excerpt.clone(), folder_id.clone());

        // Check that the post was created with the correct values
        assert_eq!(post.title, title);
        assert_eq!(post.category, category);
        assert_eq!(post.excerpt, excerpt);
        assert_eq!(post.folder_id, folder_id);

        // Check that the ID is not nil (ensuring Uuid::new_v4() worked)
        assert!(!post.id.is_nil());

        // Check that dates and timestamps are set
        assert!(post.created_at.is_some());
        assert!(post.updated_at.is_some());
    }

    #[test]
    fn test_post_new_without_folder_id() {
        let title = "Test Title".to_string();
        let category = "Test Category".to_string();
        let excerpt = "Test excerpt".to_string();
        let folder_id = None;

        let post = Post::new(title.clone(), category.clone(), excerpt.clone(), folder_id);

        assert_eq!(post.title, title);
        assert_eq!(post.category, category);
        assert_eq!(post.excerpt, excerpt);
        assert_eq!(post.folder_id, None);

        assert!(!post.id.is_nil());
        assert!(post.created_at.is_some());
        assert!(post.updated_at.is_some());
    }
}