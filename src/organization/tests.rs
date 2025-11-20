#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::model::{OrganizationMember, CreateMemberRequest, UpdateMemberRequest};

    #[test]
    fn test_organization_member_serialization() {
        let member = OrganizationMember {
            id: 1,
            name: Some("Test User".to_string()),
            position: "Manager".to_string(),
            photo: Some("photo.jpg".to_string()),
            parent_id: None,
            x: 100,
            y: 200,
            role: "lurah".to_string(),
        };

        let json = serde_json::to_string(&member).unwrap();
        let deserialized: OrganizationMember = serde_json::from_str(&json).unwrap();

        assert_eq!(member.id, deserialized.id);
        assert_eq!(member.name, deserialized.name);
        assert_eq!(member.position, deserialized.position);
    }

    #[test]
    fn test_create_member_request_deserialization() {
        let json = r#"{
            "name": "New Member",
            "position": "Staff",
            "photo": "new.jpg",
            "parent_id": 1,
            "x": 50,
            "y": 75,
            "role": "staf"
        }"#;

        let request: CreateMemberRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "New Member");
        assert_eq!(request.position, "Staff");
        assert_eq!(request.parent_id, Some(1));
    }

    #[test]
    fn test_update_member_request_partial() {
        let json = r#"{
            "name": "Updated Name"
        }"#;

        let request: UpdateMemberRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert_eq!(request.position, None);
        assert_eq!(request.x, None);
    }

    #[test]
    fn test_members_list_serialization() {
        let members = vec![
            OrganizationMember {
                id: 1,
                name: Some("Leader".to_string()),
                position: "Lurah".to_string(),
                photo: Some("leader.jpg".to_string()),
                parent_id: None,
                x: 0,
                y: 0,
                role: "lurah".to_string(),
            },
            OrganizationMember {
                id: 2,
                name: Some("Secretary".to_string()),
                position: "Sekretaris".to_string(),
                photo: Some("sec.jpg".to_string()),
                parent_id: Some(1),
                x: 100,
                y: 100,
                role: "sekretaris".to_string(),
            },
        ];

        let json = serde_json::to_vec(&members).unwrap();
        let deserialized: Vec<OrganizationMember> = serde_json::from_slice(&json).unwrap();

        assert_eq!(members.len(), deserialized.len());
        assert_eq!(members[0].id, deserialized[0].id);
        assert_eq!(members[1].parent_id, deserialized[1].parent_id);
    }
}
