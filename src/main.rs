mod diary_entry;
mod diary_state;
mod ui;

use color_eyre::eyre::{eyre, Result};
use diary_state::DiaryState;
use ui::{Action, UI};

fn main() -> Result<()> {
    color_eyre::install()?;

    // let mut diary_state = DiaryState::new();
    let mut diary_state = match DiaryState::load_from_file() {
        Ok(state) => state,
        Err(e) => {
            if e.to_string().contains("No such file or directory") {
                println!("No existing diary file found. Creating a new diary.");
                DiaryState::new()
            } else {
                return Err(eyre!("Failed to load diary: {}", e));
            }
        }
    };
    let mut ui = UI::new()?;

    loop {
        ui.display(&diary_state)?;

        if let Some(action) = ui.handle_input(&diary_state)? {
            match action {
                Action::Write => {
                    let entry = ui.get_new_entry()?;
                    diary_state.add_entry(entry);
                }
                Action::View => {
                    ui.view_entries(&diary_state)?;
                }
                Action::Edit => {
                    if let Some(entry) = ui.select_entry_to_edit(&diary_state)? {
                        let updated_entry = ui.edit_entry(&entry)?;
                        diary_state.update_entry(updated_entry);
                    }
                }
                Action::Delete => {
                    if let Some(entry) = ui.select_entry_to_delete(&diary_state)? {
                        diary_state.delete_entry(entry.id);
                    }
                }
                Action::Search => {
                    let query = ui.get_search_query()?;
                    let results = diary_state.search_entries(&query);
                    ui.display_search_results(&results)?;
                }
                Action::Quit => break,
            }
        }
    }

    Ok(())
}
