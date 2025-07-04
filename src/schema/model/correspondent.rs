use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[readonly::make]
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Correspondent {
    #[readonly]
    pub id: i32,
    #[readonly]
    pub slug: String,
    #[serde(default)]
    #[readonly]
    pub document_count: i32,
    #[readonly]
    pub last_correspondence: Option<String>,
    #[serde(default)]
    #[readonly]
    pub user_can_change: bool,
    pub name: String,
    #[serde(rename = "match")]
    pub matches: String,
    pub matching_algorithm: super::MatchingAlgorithm,
    #[serde(default = "const_true")]
    pub is_insensitive: bool,
    pub owner: i32,
    pub permissions: super::Permissions,
}

fn const_true() -> bool {
    true
}
