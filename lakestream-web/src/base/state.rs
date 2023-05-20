use crate::StringVault;

#[derive(Default, Clone)]
pub struct GlobalState {
    pub vault: Option<StringVault>,
    pub previous_url: String,
}
