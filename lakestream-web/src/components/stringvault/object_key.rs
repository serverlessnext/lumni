use super::error::SecureStringError;

#[derive(Clone, PartialEq, Debug)]
pub struct ObjectKey {
    tag: String,
    id: String,
}

impl ObjectKey {
    pub fn new(tag: &str, id: &str) -> Result<Self, SecureStringError> {
        if tag.is_empty() || id.is_empty() {
            return Err(SecureStringError::InvalidArgument(String::from(
                "Tag and ID must not be empty",
            )));
        }

        Ok(Self {
            tag: tag.to_string(),
            id: id.to_string(),
        })
    }

    pub fn new_with_form_tag(id: &str) -> Result<Self, SecureStringError> {
        if id.is_empty() {
            return Err(SecureStringError::InvalidArgument(String::from(
                "ID must not be empty",
            )));
        }

        Ok(Self {
            tag: "FORM".to_string(),
            id: id.to_string(),
        })
    }

    pub fn tag(&self) -> String {
        self.tag.clone()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    use super::*;

    #[wasm_bindgen_test]
    fn test_object_key_new() {
        let object_key = ObjectKey::new("test", "test_id").unwrap();
        assert_eq!(object_key.tag(), "test");
        assert_eq!(object_key.id(), "test_id");
    }

    #[wasm_bindgen_test]
    fn test_object_key_new_with_form_tag() {
        let object_key = ObjectKey::new_with_form_tag("test_id").unwrap();
        assert_eq!(object_key.tag(), "FORM");
        assert_eq!(object_key.id(), "test_id");
    }

    #[wasm_bindgen_test]
    async fn test_invalid_object_key_creation() {
        // Check that ObjectKey::new returns an error when given an empty id
        let object_key_empty_id = ObjectKey::new("test_tag", "");
        assert!(
            object_key_empty_id.is_err(),
            "Successfully created ObjectKey with empty id"
        );

        // Check that ObjectKey::new returns an error when given an empty tag
        let object_key_empty_tag = ObjectKey::new("", "test_id");
        assert!(
            object_key_empty_tag.is_err(),
            "Successfully created ObjectKey with empty tag"
        );
    }
}
