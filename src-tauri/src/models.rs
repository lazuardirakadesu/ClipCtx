use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    Code,
    Json,
    Url,
    Color,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Text => "text",
            ContentType::Code => "code",
            ContentType::Json => "json",
            ContentType::Url => "url",
            ContentType::Color => "color",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "code" => ContentType::Code,
            "json" => ContentType::Json,
            "url" => ContentType::Url,
            "color" => ContentType::Color,
            _ => ContentType::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: i64,
    pub content: String,
    pub content_hash: String,
    pub content_type: ContentType,
    pub source_app: String,
    pub window_title: String,
    pub is_pinned: bool,
    pub created_at: i64,
    pub last_used_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppFilterItem {
    pub source_app: String,
    pub count: i64,
}
