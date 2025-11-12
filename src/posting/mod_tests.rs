#[cfg(test)]
mod tests {
    use crate::posting::models::Post;
    use uuid::Uuid;

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
}