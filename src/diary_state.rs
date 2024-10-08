use crate::diary_entry::DiaryEntry;
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct DiaryState {
    entries: Vec<DiaryEntry>,
    next_id: usize,
}

impl DiaryState {
    pub fn new() -> Self {
        DiaryState {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_entry(&mut self, mut entry: DiaryEntry) {
        entry.id = self.next_id;
        self.next_id += 1;
        self.entries.push(entry);
        self.save_to_file().unwrap();
    }

    pub fn update_entry(&mut self, updated_entry: DiaryEntry) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == updated_entry.id) {
            *entry = updated_entry;
            self.save_to_file().unwrap();
        }
    }

    pub fn delete_entry(&mut self, id: usize) {
        self.entries.retain(|e| e.id != id);
        self.save_to_file().unwrap();
    }

    pub fn get_entries(&self) -> &Vec<DiaryEntry> {
        &self.entries
    }

    pub fn search_entries(&self, query: &str) -> Vec<DiaryEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.content.to_lowercase().contains(&query.to_lowercase())
                    || e.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query.to_lowercase()))
            })
            .cloned()
            .collect()
    }

    pub fn save_to_file(&self) -> Result<()> {
        let serialized = serde_json::to_string(&self)?;
        fs::write("diary_entries.json", serialized)?;
        Ok(())
    }

    pub fn load_from_file() -> Result<Self> {
        let serialized = fs::read_to_string("diary_entries.json")?;
        let diary_state: DiaryState = serde_json::from_str(&serialized)?;
        Ok(diary_state)
    }
}
