use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub id: usize,
    pub timestamp: DateTime<Local>,
    pub content: String,
    pub tags: Vec<String>,
}

impl DiaryEntry {
    pub fn new(id: usize, content: String, tags: Vec<String>) -> Self {
        DiaryEntry {
            id,
            timestamp: Local::now(),
            content,
            tags,
        }
    }
}
