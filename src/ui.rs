use crate::diary_entry::DiaryEntry;
use crate::diary_state::DiaryState;
use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use futures::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use reqwest::Client;
use serde_json::Value;
use std::{
    io::{stdout, Stdout},
    time::{Duration, Instant},
};

pub enum Action {
    Write,
    View,
    Edit,
    Delete,
    Search,
    Quit,
}

pub struct UI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    cursor_position: usize,
    cursor_visible: bool,
    last_cursor_update: Instant,
    http_client: Client,
}

impl UI {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;
        let http_client = Client::new();

        Ok(UI {
            terminal,
            cursor_position: 0,
            cursor_visible: true,
            last_cursor_update: Instant::now(),
            http_client,
        })
    }

    pub fn display(&mut self, diary_state: &DiaryState) -> Result<()> {
        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let title = Paragraph::new("Personal Diary")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(title, chunks[0]);

            let entries: Vec<ListItem> = diary_state
                .get_entries()
                .iter()
                .map(|entry| {
                    ListItem::new(vec![
                        Line::from(Span::raw(format!(
                            "[{}] {}",
                            entry.timestamp.format("%Y-%m-%d %H:%M"),
                            entry.content.lines().next().unwrap_or("")
                        ))),
                        Line::from(Span::raw(format!("Tags: {}", entry.tags.join(", ")))),
                    ])
                })
                .collect();

            let entries_list =
                List::new(entries).block(Block::default().borders(Borders::ALL).title("Entries"));
            f.render_widget(entries_list, chunks[1]);

            let controls = if diary_state.get_entries().is_empty() {
                Line::from(vec![
                    Span::raw("Press "),
                    Span::styled("w", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to write, "),
                    Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to quit"),
                ])
            } else {
                Line::from(vec![
                    Span::raw("Press "),
                    Span::styled("w", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to write, "),
                    Span::styled("v", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to view, "),
                    Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to edit, "),
                    Span::styled("d", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to delete, "),
                    Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to search, "),
                    Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to quit"),
                ])
            };
            let controls_paragraph = Paragraph::new(controls)
                .style(Style::default().fg(Color::Yellow))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(controls_paragraph, chunks[2]);
        })?;

        Ok(())
    }

    pub fn handle_input(&self, diary_state: &DiaryState) -> Result<Option<Action>> {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('w') => Ok(Some(Action::Write)),
                KeyCode::Char('q') => Ok(Some(Action::Quit)),
                KeyCode::Char('v') if !diary_state.get_entries().is_empty() => {
                    Ok(Some(Action::View))
                }
                KeyCode::Char('e') if !diary_state.get_entries().is_empty() => {
                    Ok(Some(Action::Edit))
                }
                KeyCode::Char('d') if !diary_state.get_entries().is_empty() => {
                    Ok(Some(Action::Delete))
                }
                KeyCode::Char('s') if !diary_state.get_entries().is_empty() => {
                    Ok(Some(Action::Search))
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
    async fn get_ai_prompt(&mut self) -> Result<String> {
        let mut prompt = String::new();

        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(1),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let input = Paragraph::new(prompt.clone()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Enter AI Prompt"),
            );
            f.render_widget(input, chunks[0]);

            let instructions = Paragraph::new("Press Enter to submit")
                .style(Style::default().fg(Color::Yellow))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(instructions, chunks[1]);
        })?;

        loop {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => break,
                    KeyCode::Char(c) => prompt.push(c),
                    KeyCode::Backspace => {
                        prompt.pop();
                    }
                    _ => {}
                }
            }
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(1),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let input = Paragraph::new(prompt.clone()).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Enter AI Prompt"),
                );
                f.render_widget(input, chunks[0]);

                let instructions = Paragraph::new("Press Enter to submit")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(instructions, chunks[1]);
            })?;
        }

        Ok(prompt)
    }

    async fn get_ai_response(&mut self, prompt: &str) -> Result<String> {
        let mut response = String::new();

        let request_body = serde_json::json!({
            "model": "llama3.2",
            "messages": [{"role": "user", "content": prompt}],
            "stream": true
        });

        let mut stream = self
            .http_client
            .post("http://localhost:11434/api/chat")
            .json(&request_body)
            .send()
            .await?
            .bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            if let Ok(value) = serde_json::from_str::<Value>(&chunk_str) {
                if let Some(content) = value["message"]["content"].as_str() {
                    response.push_str(content);
                    self.terminal.draw(|f| {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .margin(1)
                            .constraints([Constraint::Min(1)].as_ref())
                            .split(f.area());

                        let ai_response = Paragraph::new(response.clone())
                            .block(Block::default().borders(Borders::ALL).title("AI Response"));
                        f.render_widget(ai_response, chunks[0]);
                    })?;
                }
                if value["done"].as_bool().unwrap_or(false) {
                    break;
                }
            }
        }

        Ok(response)
    }

    // use this function if you want to generate tags by AI
    async fn _generate_tags(&mut self, content: &str) -> Result<String> {
        let prompt = format!(
            "Generate comma-separated tags for the following content: {}",
            content
        );
        let tags = self.get_ai_response(&prompt).await?;
        Ok(tags)
    }

    pub async fn get_new_entry(&mut self) -> Result<DiaryEntry> {
        let mut content = String::new();
        let mut tags = String::new();

        self.cursor_position = 0;
        let mut last_content_update = Instant::now();
        let mut ai_prompt_mode = false;

        loop {
            let now = Instant::now();
            let should_update_cursor =
                now.duration_since(self.last_cursor_update) >= Duration::from_millis(500);
            let should_redraw = should_update_cursor
                || now.duration_since(last_content_update) < Duration::from_millis(50);

            if should_redraw {
                self.terminal.draw(|f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [
                                Constraint::Length(3),
                                Constraint::Min(10),
                                Constraint::Length(3),
                                Constraint::Length(3),
                            ]
                            .as_ref(),
                        )
                        .split(f.area());

                    let title = Paragraph::new("New Diary Entry")
                        .style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                        .alignment(ratatui::layout::Alignment::Center);
                    f.render_widget(title, chunks[0]);

                    let content_with_cursor = if self.cursor_visible {
                        let mut content_clone = content.clone();
                        content_clone.insert(self.cursor_position, '|');
                        content_clone
                    } else {
                        content.clone()
                    };

                    let content_input = Paragraph::new(content_with_cursor)
                        .block(Block::default().borders(Borders::ALL).title("Content"));
                    f.render_widget(content_input, chunks[1]);

                    let mode_info = if ai_prompt_mode {
                        "AI Prompt Mode (Press 'Esc' to exit)"
                    } else {
                        "Manual Typing Mode (Press '*' for AI assistance)"
                    };
                    let mode_paragraph = Paragraph::new(mode_info)
                        .style(Style::default().fg(Color::Yellow))
                        .alignment(ratatui::layout::Alignment::Center);
                    f.render_widget(mode_paragraph, chunks[2]);

                    let instructions = Paragraph::new("Press Esc to finish")
                        .style(Style::default().fg(Color::Yellow))
                        .alignment(ratatui::layout::Alignment::Center);
                    f.render_widget(instructions, chunks[3]);
                })?;

                if should_update_cursor {
                    self.cursor_visible = !self.cursor_visible;
                    self.last_cursor_update = now;
                }
            }

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => {
                            if ai_prompt_mode {
                                ai_prompt_mode = false;
                            } else {
                                break;
                            }
                        }
                        KeyCode::Char('*') => {
                            // ai_prompt_mode = true;
                            let prompt = self.get_ai_prompt().await?;
                            let ai_response = self.get_ai_response(&prompt).await?;
                            content.push_str(&ai_response);
                            self.cursor_position = content.len();
                            last_content_update = Instant::now();
                            ai_prompt_mode = false;
                        }
                        KeyCode::Char(c) if !ai_prompt_mode => {
                            content.insert(self.cursor_position, c);
                            self.cursor_position += 1;
                            last_content_update = Instant::now();
                        }
                        KeyCode::Backspace if !ai_prompt_mode => {
                            if self.cursor_position > 0 {
                                content.remove(self.cursor_position - 1);
                                self.cursor_position -= 1;
                                last_content_update = Instant::now();
                            }
                        }
                        KeyCode::Delete if !ai_prompt_mode => {
                            if self.cursor_position < content.len() {
                                content.remove(self.cursor_position);
                                last_content_update = Instant::now();
                            }
                        }
                        KeyCode::Left if !ai_prompt_mode => {
                            if self.cursor_position > 0 {
                                self.cursor_position -= 1;
                                last_content_update = Instant::now();
                            }
                        }
                        KeyCode::Right if !ai_prompt_mode => {
                            if self.cursor_position < content.len() {
                                self.cursor_position += 1;
                                last_content_update = Instant::now();
                            }
                        }
                        KeyCode::Up => {
                            let current_line_start = content[..self.cursor_position]
                                .rfind('\n')
                                .map(|i| i + 1)
                                .unwrap_or(0);
                            if let Some(prev_line_start) =
                                content[..current_line_start.saturating_sub(1)].rfind('\n')
                            {
                                let prev_line_length = current_line_start - prev_line_start - 1;
                                let current_column = self.cursor_position - current_line_start;
                                self.cursor_position =
                                    prev_line_start + 1 + current_column.min(prev_line_length);
                            }
                            last_content_update = Instant::now();
                        }
                        KeyCode::Down => {
                            if let Some(next_line_start) =
                                content[self.cursor_position..].find('\n')
                            {
                                let current_line_start = content[..self.cursor_position]
                                    .rfind('\n')
                                    .map(|i| i + 1)
                                    .unwrap_or(0);
                                let current_column = self.cursor_position - current_line_start;
                                let next_line_end = content
                                    [self.cursor_position + next_line_start + 1..]
                                    .find('\n')
                                    .map(|i| self.cursor_position + next_line_start + 1 + i)
                                    .unwrap_or(content.len());
                                let next_line_length =
                                    next_line_end - (self.cursor_position + next_line_start + 1);
                                self.cursor_position = self.cursor_position
                                    + next_line_start
                                    + 1
                                    + current_column.min(next_line_length);
                                last_content_update = Instant::now();
                            }
                        }
                        KeyCode::Enter if !ai_prompt_mode => {
                            content.insert(self.cursor_position, '\n');
                            self.cursor_position += 1;
                            last_content_update = Instant::now();
                        }
                        _ => {}
                    }
                }
            }
        }

        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let content_input = Paragraph::new(content.clone())
                .block(Block::default().borders(Borders::ALL).title("Content"));
            f.render_widget(content_input, chunks[1]);
        })?;

        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let tags_input = Paragraph::new(tags.clone()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Tags (comma-separated)"),
            );
            f.render_widget(tags_input, chunks[2]);

            let instructions = Paragraph::new("Press Esc to save")
                .style(Style::default().fg(Color::Yellow))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(instructions, chunks[3]);
        })?;

        loop {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Char(c) => {
                        tags.push(c);
                    }
                    KeyCode::Backspace => {
                        tags.pop();
                    }
                    _ => {}
                }
            }
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(10),
                            Constraint::Length(3),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let tags_input = Paragraph::new(tags.clone()).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Tags (comma-separated)"),
                );
                f.render_widget(tags_input, chunks[2]);

                let instructions = Paragraph::new("Press Esc to save")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(instructions, chunks[3]);
            })?;
        }

        // Automatically generate tags based on content
        // tags = self.generate_tags(&content).await?;
        //
        // let tag_list = tags.split(',').map(|s| s.trim().to_string()).collect();
        // Ok(DiaryEntry::new(0, content, tag_list))

        let tag_list = tags.split(',').map(|s| s.trim().to_string()).collect();
        Ok(DiaryEntry::new(0, content, tag_list))
    }

    pub fn view_entries(&mut self, diary_state: &DiaryState) -> Result<()> {
        let entries = diary_state.get_entries();
        let mut selected_index = 0;

        loop {
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(10),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let title = Paragraph::new("View Entries")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(title, chunks[0]);

                let items: Vec<ListItem> = entries
                    .iter()
                    .map(|e| {
                        ListItem::new(vec![
                            Line::from(Span::raw(format!(
                                "[{}] {}",
                                e.timestamp.format("%Y-%m-%d %H:%M"),
                                e.content.lines().next().unwrap_or("")
                            ))),
                            Line::from(Span::raw(format!("Tags: {}", e.tags.join(", ")))),
                        ])
                    })
                    .collect();

                let entries_list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Entries"))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                    .highlight_symbol("> ");

                f.render_stateful_widget(
                    entries_list,
                    chunks[1],
                    &mut ListState::default().with_selected(Some(selected_index)),
                );

                let instructions =
                    Paragraph::new("Up/Down: Navigate, Enter: View full entry, Esc: Back")
                        .style(Style::default().fg(Color::Yellow))
                        .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(instructions, chunks[2]);
            })?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected_index = selected_index.saturating_sub(1),
                    KeyCode::Down => {
                        if selected_index < entries.len() - 1 {
                            selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        self.view_full_entry(&entries[selected_index])?;
                    }
                    KeyCode::Esc => break,
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn view_full_entry(&mut self, entry: &DiaryEntry) -> Result<()> {
        loop {
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(10),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let title = Paragraph::new(format!(
                    "Entry from {}",
                    entry.timestamp.format("%Y-%m-%d %H:%M"),
                ))
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(title, chunks[0]);

                let content = Paragraph::new(entry.content.clone())
                    .block(Block::default().borders(Borders::ALL).title("Content"));
                f.render_widget(content, chunks[1]);

                let instructions = Paragraph::new("Esc: Back")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(instructions, chunks[2]);
            })?;

            if let Event::Key(_) = event::read()? {
                break;
            }
        }

        Ok(())
    }

    pub fn select_entry_to_edit(&mut self, diary_state: &DiaryState) -> Result<Option<DiaryEntry>> {
        let entries = diary_state.get_entries();
        let mut selected_index = 0;

        loop {
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(10),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let title = Paragraph::new("Select Entry to Edit")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(title, chunks[0]);

                let items: Vec<ListItem> = entries
                    .iter()
                    .map(|e| {
                        ListItem::new(vec![
                            Line::from(Span::raw(format!(
                                "[{}] {}",
                                e.timestamp.format("%Y-%m-%d %H:%M"),
                                e.content.lines().next().unwrap_or("")
                            ))),
                            Line::from(Span::raw(format!("Tags: {}", e.tags.join(", ")))),
                        ])
                    })
                    .collect();

                let entries_list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Entries"))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                    .highlight_symbol("> ");

                f.render_stateful_widget(
                    entries_list,
                    chunks[1],
                    &mut ListState::default().with_selected(Some(selected_index)),
                );

                let instructions = Paragraph::new("Up/Down: Navigate, Enter: Select, Esc: Cancel")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(instructions, chunks[2]);
            })?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected_index = selected_index.saturating_sub(1),
                    KeyCode::Down => {
                        if selected_index < entries.len() - 1 {
                            selected_index += 1;
                        }
                    }
                    KeyCode::Enter => return Ok(Some(entries[selected_index].clone())),
                    KeyCode::Esc => return Ok(None),
                    _ => {}
                }
            }
        }
    }

    pub fn edit_entry(&mut self, entry: &DiaryEntry) -> Result<DiaryEntry> {
        let mut content = entry.content.clone();
        let mut tags = entry.tags.join(", ");
        self.cursor_position = content.len();

        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let title = Paragraph::new("Edit Diary Entry")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(title, chunks[0]);

            let content_input = Paragraph::new(content.clone())
                .block(Block::default().borders(Borders::ALL).title("Content"));
            f.render_widget(content_input, chunks[1]);

            let tags_input = Paragraph::new(tags.clone()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Tags (comma-separated)"),
            );
            f.render_widget(tags_input, chunks[2]);

            let instructions = Paragraph::new("Press Esc to finish")
                .style(Style::default().fg(Color::Yellow))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(instructions, chunks[3]);
        })?;

        loop {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Char(c) => {
                        content.push(c);
                    }
                    KeyCode::Backspace => {
                        content.pop();
                    }
                    KeyCode::Enter => {
                        content.push('\n');
                    }
                    _ => {}
                }
            }
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(10),
                            Constraint::Length(3),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let content_input = Paragraph::new(content.clone())
                    .block(Block::default().borders(Borders::ALL).title("Content"));
                f.render_widget(content_input, chunks[1]);
            })?;
        }
        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let tags_input = Paragraph::new(tags.clone()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Tags (comma-separated)"),
            );
            f.render_widget(tags_input, chunks[2]);
        })?;

        loop {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Char(c) => {
                        tags.push(c);
                    }
                    KeyCode::Backspace => {
                        tags.pop();
                    }
                    _ => {}
                }
            }
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(10),
                            Constraint::Length(3),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let tags_input = Paragraph::new(tags.clone()).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Tags (comma-separated)"),
                );
                f.render_widget(tags_input, chunks[2]);
            })?;
        }

        let tag_list = tags.split(',').map(|s| s.trim().to_string()).collect();
        Ok(DiaryEntry {
            id: entry.id,
            timestamp: entry.timestamp,
            content,
            tags: tag_list,
        })
    }

    pub fn select_entry_to_delete(
        &mut self,
        diary_state: &DiaryState,
    ) -> Result<Option<DiaryEntry>> {
        let entries = diary_state.get_entries();
        let mut selected_index = 0;

        loop {
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(10),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let title = Paragraph::new("Select Entry to Delete")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(title, chunks[0]);

                let items: Vec<ListItem> = entries
                    .iter()
                    .map(|e| {
                        ListItem::new(vec![
                            Line::from(Span::raw(format!(
                                "[{}] {}",
                                e.timestamp.format("%Y-%m-%d %H:%M"),
                                e.content.lines().next().unwrap_or("")
                            ))),
                            Line::from(Span::raw(format!("Tags: {}", e.tags.join(", ")))),
                        ])
                    })
                    .collect();

                let entries_list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Entries"))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                    .highlight_symbol("> ");

                f.render_stateful_widget(
                    entries_list,
                    chunks[1],
                    &mut ListState::default().with_selected(Some(selected_index)),
                );

                let instructions = Paragraph::new("Up/Down: Navigate, Enter: Select, Esc: Cancel")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(instructions, chunks[2]);
            })?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected_index = selected_index.saturating_sub(1),
                    KeyCode::Down => {
                        if selected_index < entries.len() - 1 {
                            selected_index += 1;
                        }
                    }
                    KeyCode::Enter => return Ok(Some(entries[selected_index].clone())),
                    KeyCode::Esc => return Ok(None),
                    _ => {}
                }
            }
        }
    }

    pub fn get_search_query(&mut self) -> Result<String> {
        let mut query = String::new();

        loop {
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Length(3),
                            Constraint::Min(1),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let title = Paragraph::new("Search Entries")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(title, chunks[0]);

                let search_input = Paragraph::new(query.clone())
                    .block(Block::default().borders(Borders::ALL).title("Search Query"));
                f.render_widget(search_input, chunks[1]);

                let instructions = Paragraph::new("Enter: Submit, Esc: Cancel")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(instructions, chunks[2]);
            })?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => break,
                    KeyCode::Char(c) => {
                        query.push(c);
                    }
                    KeyCode::Backspace => {
                        query.pop();
                    }
                    KeyCode::Esc => return Ok(String::new()),
                    _ => {}
                }
            }
        }

        Ok(query)
    }

    pub fn display_search_results(&mut self, results: &[DiaryEntry]) -> Result<()> {
        let mut selected_index = 0;

        loop {
            self.terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(10),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let title = Paragraph::new("Search Results")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(title, chunks[0]);

                let items: Vec<ListItem> = results
                    .iter()
                    .map(|e| {
                        ListItem::new(vec![
                            Line::from(Span::raw(format!(
                                "[{}] {}",
                                e.timestamp.format("%Y-%m-%d %H:%M"),
                                e.content.lines().next().unwrap_or("")
                            ))),
                            Line::from(Span::raw(format!("Tags: {}", e.tags.join(", ")))),
                        ])
                    })
                    .collect();

                let results_list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Results"))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                    .highlight_symbol("> ");

                f.render_stateful_widget(
                    results_list,
                    chunks[1],
                    &mut ListState::default().with_selected(Some(selected_index)),
                );

                let instructions =
                    Paragraph::new("Up/Down: Navigate, Enter: View full entry, Esc: Back")
                        .style(Style::default().fg(Color::Yellow))
                        .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(instructions, chunks[2]);
            })?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected_index = selected_index.saturating_sub(1),
                    KeyCode::Down => {
                        if selected_index < results.len() - 1 {
                            selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        self.view_full_entry(&results[selected_index])?;
                    }
                    KeyCode::Esc => break,
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

impl Drop for UI {
    fn drop(&mut self) {
        disable_raw_mode().unwrap();
        stdout().execute(LeaveAlternateScreen).unwrap();
    }
}
