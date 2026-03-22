use crate::components::baker::storage::v1::{BackgroundSettings, ChatHeadStyle};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub(crate) struct LegacyContact {
    pub id: usize,
    pub unread_count: usize,
    #[serde(default)]
    pub chat_head_style: ChatHeadStyle,
}

#[derive(Deserialize)]
pub(crate) struct LegacyMessage {
    pub id: usize,
    pub sender_id: usize,
    pub content: String,
    pub timestamp: String,
    #[serde(default)]
    pub animate: bool,
}

#[derive(Deserialize)]
pub(crate) struct LegacyOperator {
    pub id: usize,
    pub name: String,
    pub avatar_url: String,
}

#[derive(Deserialize)]
pub(crate) struct LegacyUserProfile {
    pub name: String,
    pub avatar_url: String,
}

#[derive(Deserialize)]
pub(crate) struct LegacyAppState {
    pub user_profile: LegacyUserProfile,
    pub contacts: Vec<LegacyContact>,
    pub messages: HashMap<usize, Vec<LegacyMessage>>,
    pub operators: Vec<LegacyOperator>,
    #[serde(default)]
    pub background: BackgroundSettings,
}
