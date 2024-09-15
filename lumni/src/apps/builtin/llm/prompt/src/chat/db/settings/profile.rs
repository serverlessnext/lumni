use futures::Future;
use lumni::api::error::ApplicationError;
use serde_json::Value as JsonValue;

use super::{MaskMode, UserProfileDbHandler};
use crate::external as lumni;

const SECTION: &str = "profile";

#[derive(Debug, Clone, PartialEq)]
pub struct UserProfile {
    pub id: i64,
    pub name: String,
}

impl UserProfileDbHandler {
    pub fn create_profile(
        &mut self,
        name: String,
        parameters: JsonValue,
    ) -> impl Future<Output = Result<UserProfile, ApplicationError>> + '_ {
        async move {
            let item = self
                .create_configuration_item(name, SECTION, parameters)
                .await?;
            Ok(UserProfile {
                id: item.id,
                name: item.name,
            })
        }
    }

    pub async fn delete_profile(
        &self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        self.delete_configuration_item(&profile.into()).await
    }

    pub async fn update_profile(
        &mut self,
        profile: &UserProfile,
        new_settings: &JsonValue,
    ) -> Result<(), ApplicationError> {
        self.update_configuration_item(&profile.into(), new_settings)
            .await
    }

    pub async fn rename_profile(
        &self,
        profile: &UserProfile,
        new_name: &str,
    ) -> Result<(), ApplicationError> {
        self.rename_configuration_item(&profile.into(), new_name)
            .await
    }

    pub async fn set_default_profile(
        &self,
        profile: &UserProfile,
    ) -> Result<(), ApplicationError> {
        self.set_default_configuration_item(&profile.into()).await
    }

    pub async fn list_profiles(
        &self,
    ) -> Result<Vec<UserProfile>, ApplicationError> {
        let items = self.list_configuration_items(SECTION).await?;
        Ok(items
            .into_iter()
            .map(|item| UserProfile {
                id: item.id,
                name: item.name,
            })
            .collect())
    }

    pub async fn get_profile_settings(
        &self,
        profile: &UserProfile,
        mask_mode: MaskMode,
    ) -> Result<JsonValue, ApplicationError> {
        self.get_configuration_parameters(&profile.into(), mask_mode)
            .await
    }

    pub async fn get_profile_by_id(
        &self,
        id: i64,
    ) -> Result<Option<UserProfile>, ApplicationError> {
        let item = self.get_configuration_item_by_id(id).await?;
        match item {
            Some(item) => Ok(Some(UserProfile {
                id: item.id,
                name: item.name,
            })),
            _ => Ok(None),
        }
    }

    pub async fn get_profiles_by_name(
        &self,
        name: &str,
    ) -> Result<Vec<UserProfile>, ApplicationError> {
        let items = self.get_configuration_items_by_name(SECTION, name).await?;
        Ok(items
            .into_iter()
            .map(|item| UserProfile {
                id: item.id,
                name: item.name,
            })
            .collect())
    }
}
