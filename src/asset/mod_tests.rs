#[cfg(test)]
mod tests {
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