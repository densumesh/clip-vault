use chrono_humanize::{Accuracy, HumanTime, Tense};
use clip_vault_core::{ClipboardItem, ClipboardItemWithTimestamp, Result, SqliteVault, Vault};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEvent, MouseEventKind};
use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame, Terminal,
};
use std::io;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{fs, process::Command};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::{SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Preview,
}

pub struct App {
    vault: SqliteVault,
    items: Vec<ClipboardItemWithTimestamp>,
    filtered_items: Vec<ClipboardItemWithTimestamp>,
    list_state: ListState,
    mode: Mode,
    search_query: String,
    search_cursor: usize,
    preview_text: String,
    preview_lines: Vec<ratatui::text::Line<'static>>,
    preview_offset: usize,
    should_quit: bool,
    status_message: String,
    scrollbar_state: ScrollbarState,
}

impl App {
    pub fn new(vault: SqliteVault) -> Result<Self> {
        let mut app = Self {
            vault,
            items: Vec::new(),
            filtered_items: Vec::new(),
            list_state: ListState::default(),
            mode: Mode::Normal,
            search_query: String::new(),
            search_cursor: 0,
            preview_text: String::new(),
            preview_lines: Vec::new(),
            preview_offset: 0,
            should_quit: false,
            status_message: "Welcome to Clip Vault! Press ? for help".to_string(),
            scrollbar_state: ScrollbarState::default(),
        };
        app.load_items()?;
        if !app.items.is_empty() {
            app.list_state.select(Some(0));
        }
        app.update_scrollbar();
        Ok(app)
    }

    pub fn load_items(&mut self) -> Result<()> {
        self.items = self.vault.list(None)?;
        self.apply_filter();
        Ok(())
    }

    fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_items = self.items.clone();
        } else {
            // Use the vault's search functionality for consistency
            match self.vault.search(&self.search_query, None) {
                Ok(results) => self.filtered_items = results,
                Err(_) => {
                    // Fallback to simple text matching if search fails
                    self.filtered_items = self
                        .items
                        .iter()
                        .filter(|item_with_ts| match &item_with_ts.item {
                            ClipboardItem::Text(text) => text
                                .to_lowercase()
                                .contains(&self.search_query.to_lowercase()),
                            ClipboardItem::Image(_) => {
                                // For images, search in the query for "image"
                                self.search_query.to_lowercase().contains("image")
                            }
                        })
                        .cloned()
                        .collect();
                }
            }
        }

        // Reset selection to first item if available
        if self.filtered_items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
        self.update_scrollbar();
    }

    fn update_scrollbar(&mut self) {
        self.scrollbar_state = self
            .scrollbar_state
            .content_length(self.filtered_items.len());
        if let Some(selected) = self.list_state.selected() {
            self.scrollbar_state = self.scrollbar_state.position(selected);
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match self.mode {
                        Mode::Normal => self.handle_normal_input(key.code)?,
                        Mode::Search => self.handle_search_input(key.code),
                        Mode::Preview => self.handle_preview_input(key.code, terminal)?,
                    }
                }
            } else if let Event::Mouse(mouse) = event::read()? {
                self.handle_mouse_input(mouse);
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_normal_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.next_item(),
            KeyCode::Char('k') | KeyCode::Up => self.previous_item(),
            KeyCode::Char('g') => self.go_to_top(),
            KeyCode::Char('G') => self.go_to_bottom(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::Char('/') => self.enter_search_mode(),
            KeyCode::Char('c') => self.copy_selected_item()?,
            KeyCode::Char('d') => self.delete_selected_item()?,
            KeyCode::Enter | KeyCode::Char(' ') => self.preview_selected_item(),
            KeyCode::Char('r') => self.refresh_items()?,
            KeyCode::Char('?') => self.show_help(),
            _ => {}
        }
        Ok(())
    }

    fn handle_search_input(&mut self, key: KeyCode) {
        match key {
            // Navigation within filtered list
            KeyCode::Up => self.previous_item(),
            KeyCode::Down => self.next_item(),

            // Search-specific controls
            KeyCode::Esc => self.exit_search_mode(),
            KeyCode::Enter => self.execute_search(),
            KeyCode::Backspace => self.delete_search_char(),
            KeyCode::Left => self.move_search_cursor_left(),
            KeyCode::Right => self.move_search_cursor_right(),

            // Text input
            KeyCode::Char(c) => self.add_search_char(c),

            _ => {}
        }
    }

    fn handle_preview_input<B: Backend>(
        &mut self,
        key: KeyCode,
        terminal: &mut Terminal<B>,
    ) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => self.exit_preview_mode(),
            KeyCode::Char('c') => self.copy_selected_item()?,
            KeyCode::Char('d') => self.delete_selected_item()?,
            KeyCode::Char('e') => self.edit_selected_item(terminal)?,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.preview_offset > 0 {
                    self.preview_offset -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.preview_offset + 1 < self.preview_lines.len() {
                    self.preview_offset += 1;
                }
            }
            KeyCode::PageUp => {
                self.preview_offset = self.preview_offset.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.preview_offset =
                    (self.preview_offset + 10).min(self.preview_lines.len().saturating_sub(1));
            }
            _ => {}
        }
        Ok(())
    }

    fn next_item(&mut self) {
        if !self.filtered_items.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= self.filtered_items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
            self.update_scrollbar();
        }
    }

    fn previous_item(&mut self) {
        if !self.filtered_items.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.filtered_items.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
            self.update_scrollbar();
        }
    }

    fn go_to_top(&mut self) {
        if !self.filtered_items.is_empty() {
            self.list_state.select(Some(0));
            self.update_scrollbar();
        }
    }

    fn go_to_bottom(&mut self) {
        if !self.filtered_items.is_empty() {
            self.list_state.select(Some(self.filtered_items.len() - 1));
            self.update_scrollbar();
        }
    }

    fn page_down(&mut self) {
        if !self.filtered_items.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => (i + 10).min(self.filtered_items.len() - 1),
                None => 0,
            };
            self.list_state.select(Some(i));
            self.update_scrollbar();
        }
    }

    fn page_up(&mut self) {
        if !self.filtered_items.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => i.saturating_sub(10),
                None => 0,
            };
            self.list_state.select(Some(i));
            self.update_scrollbar();
        }
    }

    fn enter_search_mode(&mut self) {
        self.mode = Mode::Search;
        self.search_query.clear();
        self.search_cursor = 0;
        // Reset to show all items when entering search mode
        self.apply_filter();
        self.status_message =
            "Search mode - type to search, Enter to exit, Esc to cancel".to_string();
    }

    fn exit_search_mode(&mut self) {
        self.mode = Mode::Normal;
        self.search_query.clear();
        self.search_cursor = 0;
        self.apply_filter();
        self.status_message = "Welcome to Clip Vault! Press ? for help".to_string();
    }

    fn execute_search(&mut self) {
        self.mode = Mode::Normal;
        // Search is already applied, just exit search mode
        let count = self.filtered_items.len();
        self.status_message = if self.search_query.is_empty() {
            "Showing all items".to_string()
        } else {
            format!("Found {} items matching '{}'", count, self.search_query)
        };
    }

    fn delete_search_char(&mut self) {
        if self.search_cursor > 0 {
            self.search_cursor -= 1;
            self.search_query.remove(self.search_cursor);
            // Auto-apply search as user deletes
            self.apply_filter();
            let count = self.filtered_items.len();
            self.status_message = if self.search_query.is_empty() {
                "Search mode - type to search, Enter to exit, Esc to cancel".to_string()
            } else {
                format!(
                    "Found {} items matching '{}' - Enter to exit, Esc to cancel",
                    count, self.search_query
                )
            };
        }
    }

    fn move_search_cursor_left(&mut self) {
        self.search_cursor = self.search_cursor.saturating_sub(1);
    }

    fn move_search_cursor_right(&mut self) {
        if self.search_cursor < self.search_query.len() {
            self.search_cursor += 1;
        }
    }

    fn add_search_char(&mut self, c: char) {
        self.search_query.insert(self.search_cursor, c);
        self.search_cursor += 1;
        // Auto-apply search as user types
        self.apply_filter();
        let count = self.filtered_items.len();
        self.status_message = if self.search_query.is_empty() {
            "Search mode - type to search, Enter to exit, Esc to cancel".to_string()
        } else {
            format!(
                "Found {} items matching '{}' - Enter to exit, Esc to cancel",
                count, self.search_query
            )
        };
    }

    fn copy_selected_item(&mut self) -> Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(item_with_ts) = self.filtered_items.get(selected) {
                match &item_with_ts.item {
                    ClipboardItem::Text(text) => {
                        Self::copy_text_to_clipboard(&text.clone())?;
                        self.status_message = "Copied to clipboard!".to_string();
                    }
                    ClipboardItem::Image(_) => {
                        self.status_message = "Cannot copy images in CLI mode".to_string();
                    }
                }
            }
        }
        Ok(())
    }

    fn copy_text_to_clipboard(text: &str) -> Result<()> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| clip_vault_core::Error::Io(io::Error::other(e)))?;
        clipboard
            .set_text(text)
            .map_err(|e| clip_vault_core::Error::Io(io::Error::other(e)))?;
        Ok(())
    }

    fn preview_selected_item(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(item_with_ts) = self.filtered_items.get(selected) {
                // Extract text without holding the immutable borrow during mutable operations
                let txt = match &item_with_ts.item {
                    ClipboardItem::Text(t) => Some(t.clone()),
                    ClipboardItem::Image(_) => {
                        Some("[Image content - not displayable in CLI]".to_string())
                    }
                };

                if let Some(t) = txt {
                    self.prepare_preview(&t);
                    self.mode = Mode::Preview;
                    self.status_message =
                        "Preview mode - press Esc to return, 'c' to copy".to_string();
                }
            }
        }
    }

    fn exit_preview_mode(&mut self) {
        self.mode = Mode::Normal;
        self.preview_text.clear();
        self.preview_lines.clear();
        self.preview_offset = 0;
        self.status_message = "Welcome to Clip Vault! Press ? for help".to_string();
    }

    /// Launch $EDITOR with the current item, save changes back to the vault.
    fn edit_selected_item<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let Some(selected) = self.list_state.selected() else {
            return Ok(());
        };

        let Some(item_with_ts) = self.filtered_items.get(selected) else {
            return Ok(());
        };

        let original_text = match &item_with_ts.item {
            ClipboardItem::Text(t) => t.clone(),
            ClipboardItem::Image(_) => {
                self.status_message = "Cannot edit images in CLI mode".to_string();
                return Ok(());
            }
        };
        let original_hash = item_with_ts.item.hash();

        // temp file path
        let mut path = std::env::temp_dir();
        path.push("clip_vault_edit.txt");
        fs::write(&path, &original_text)?;

        // Temporarily leave raw mode so the external $EDITOR can own the terminal.

        disable_raw_mode()?;
        terminal.clear()?;
        execute!(std::io::stdout(), DisableMouseCapture, Show)?;

        // determine editor
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let status = Command::new(editor).arg(&path).status()?;

        // Restore TUI state
        execute!(std::io::stdout(), EnableMouseCapture, Hide)?;
        enable_raw_mode()?;
        if !status.success() {
            self.status_message = "Editor aborted".into();
            return Ok(());
        }

        let new_text = fs::read_to_string(&path)?;
        if new_text == original_text {
            self.status_message = "No changes made".into();
            return Ok(());
        }

        let new_item = ClipboardItem::Text(new_text.clone());
        self.vault.update(original_hash, &new_item)?;

        // refresh lists and preview view
        self.load_items()?;
        Self::copy_text_to_clipboard(&new_text)?;
        self.prepare_preview(&new_text);
        self.status_message = "Saved changes to vault".into();
        Ok(())
    }

    fn refresh_items(&mut self) -> Result<()> {
        self.load_items()?;
        self.status_message = format!("Refreshed - {} items loaded", self.items.len());
        Ok(())
    }

    fn show_help(&mut self) {
        self.status_message = "j/↓:down k/↑:up g:top G:bottom /:live-search c:copy Space/Enter:preview r:refresh q:quit".to_string();
    }

    fn format_timestamp(timestamp: u64) -> String {
        let system_time = UNIX_EPOCH + Duration::from_nanos(timestamp);
        let now = SystemTime::now();

        // If more than 1 hour ago, show simple date/time format
        if let Ok(duration) = now.duration_since(system_time) {
            if duration.as_secs() > 3600 {
                // 1 hour
                let secs_since_epoch = system_time
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let days_since_epoch = secs_since_epoch / 86400;
                let remaining_secs = secs_since_epoch % 86400;
                let hours = remaining_secs / 3600;
                let minutes = (remaining_secs % 3600) / 60;

                // Simple date calculation from epoch days
                let mut year = 1970;
                let mut days = days_since_epoch;
                while days >= 365 {
                    if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                        if days >= 366 {
                            days -= 366;
                            year += 1;
                        } else {
                            break;
                        }
                    } else {
                        days -= 365;
                        year += 1;
                    }
                }

                let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
                let mut month = 1;
                let mut day_in_month = days + 1;

                for &month_length in &month_days {
                    let adjusted_length = if month == 2
                        && ((year % 4 == 0 && year % 100 != 0) || (year % 400 == 0))
                    {
                        29
                    } else {
                        month_length
                    };

                    if day_in_month <= adjusted_length {
                        break;
                    }
                    day_in_month -= adjusted_length;
                    month += 1;
                }

                return format!("{month:02}/{day_in_month:02} {hours:02}:{minutes:02}");
            }
        }

        // Otherwise use relative time
        let human_time = HumanTime::from(system_time);
        human_time.to_text_en(Accuracy::Rough, Tense::Past)
    }

    pub fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(1),    // Main content
                Constraint::Length(3), // Footer
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("📋 Clip Vault - Clipboard History Manager")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).title("Clip Vault"));
        f.render_widget(header, chunks[0]);

        match self.mode {
            Mode::Preview => self.render_preview(f, chunks[1]),
            _ => self.render_list(f, chunks[1]),
        }

        // Footer
        self.render_footer(f, chunks[2]);
    }

    fn render_list(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        // Split area into timestamp and content columns
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),     // Content column
                Constraint::Length(20), // Timestamp column
            ])
            .split(area);

        // Build the underlying list items first (borrows end immediately)
        let timestamp_items = self.build_timestamp_items();
        let content_items = self.build_content_items();

        // Construct the widgets without borrowing `self`
        let timestamp_list = List::new(timestamp_items)
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
                    .title("Time"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::LightBlue)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

        let content_list = List::new(content_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.list_title()),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::LightBlue)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

        // Render both lists with shared state
        f.render_stateful_widget(content_list, chunks[0], &mut self.list_state);
        f.render_stateful_widget(timestamp_list, chunks[1], &mut self.list_state);

        // Render scrollbar on the right
        self.render_scrollbar(f, area);
    }

    /// Build `ListItem`s for the timestamp column.
    fn build_timestamp_items(&self) -> Vec<ListItem<'static>> {
        self.filtered_items
            .iter()
            .map(|item_with_ts| {
                let timestamp_str = Self::format_timestamp(item_with_ts.timestamp);
                ListItem::new(Line::from(Span::styled(
                    timestamp_str,
                    Style::default().fg(Color::DarkGray),
                )))
            })
            .collect()
    }

    /// Build `ListItem`s for the content column.
    fn build_content_items(&self) -> Vec<ListItem<'static>> {
        self.filtered_items
            .iter()
            .enumerate()
            .map(|(i, item_with_ts)| {
                let content = match &item_with_ts.item {
                    ClipboardItem::Text(text) => {
                        let preview = if text.len() > 80 {
                            format!("{}...", &text[..80])
                        } else {
                            text.clone()
                        };

                        // Replace newlines with ↵ symbol for better display
                        let preview = preview.replace('\n', "↵").replace('\r', "");

                        let mut spans = vec![Span::styled(
                            format!("{:>3}. ", i + 1),
                            Style::default().fg(Color::DarkGray),
                        )];

                        // Add search highlighting if in search mode
                        if self.search_query.is_empty() {
                            spans.push(Span::raw(preview));
                        } else {
                            let search_lower = self.search_query.to_lowercase();
                            let preview_lower = preview.to_lowercase();

                            if let Some(pos) = preview_lower.find(&search_lower) {
                                // Text before match
                                if pos > 0 {
                                    spans.push(Span::raw(preview[..pos].to_string()));
                                }
                                // Highlighted match
                                spans.push(Span::styled(
                                    preview[pos..pos + self.search_query.len()].to_string(),
                                    Style::default().bg(Color::Yellow).fg(Color::Black),
                                ));
                                // Text after match
                                if pos + self.search_query.len() < preview.len() {
                                    spans.push(Span::raw(
                                        preview[pos + self.search_query.len()..].to_string(),
                                    ));
                                }
                            } else {
                                spans.push(Span::raw(preview.clone()));
                            }
                        }

                        Line::from(spans)
                    }
                    ClipboardItem::Image(data) => {
                        let mut spans = vec![Span::styled(
                            format!("{:>3}. ", i + 1),
                            Style::default().fg(Color::DarkGray),
                        )];

                        spans.push(Span::styled(
                            format!("📷 [Image: {} bytes]", data.len()),
                            Style::default().fg(Color::Blue),
                        ));

                        Line::from(spans)
                    }
                };
                ListItem::new(content)
            })
            .collect()
    }

    /// Title for the content list depending on search state.
    fn list_title(&self) -> String {
        if self.search_query.is_empty() {
            format!("Clipboard History ({} items)", self.filtered_items.len())
        } else {
            format!(
                "Search Results ({} of {} items)",
                self.filtered_items.len(),
                self.items.len()
            )
        }
    }

    /// Render the vertical scrollbar on the right of the list area.
    fn render_scrollbar(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        let scrollbar_area = ratatui::layout::Rect {
            x: area.x + area.width - 1,
            y: area.y + 1,
            width: 1,
            height: area.height - 2,
        };

        if self.filtered_items.len() > (area.height as usize - 2) {
            f.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓")),
                scrollbar_area,
                &mut self.scrollbar_state,
            );
        }
    }

    fn render_preview(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        let title = String::from("Preview (Esc to close, 'c' to copy, 'e' to edit)");

        let block = Block::default().title(title).borders(Borders::ALL);

        // Determine visible lines
        let height = area.height.saturating_sub(2) as usize; // border padding
        let end = (self.preview_offset + height).min(self.preview_lines.len());
        let slice = &self.preview_lines[self.preview_offset..end];

        let paragraph = Paragraph::new(slice.to_vec())
            .block(block)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false });

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }

    fn render_footer(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let footer_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(30)])
            .split(area);

        // Status message
        let status = match self.mode {
            Mode::Search => {
                let search_text = format!("Search: {}", self.search_query);
                let mut spans = vec![Span::raw(search_text)];

                spans.push(Span::styled("│", Style::default().fg(Color::Yellow)));
                Paragraph::new(Line::from(spans))
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::default().borders(Borders::ALL))
            }
            _ => Paragraph::new(self.status_message.clone())
                .style(Style::default().fg(Color::Green))
                .block(Block::default().borders(Borders::ALL)),
        };

        f.render_widget(status, footer_chunks[0]);

        // Help text
        let help = Paragraph::new("Press ? for help")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, footer_chunks[1]);
    }

    /// Prepare highlighted lines for preview and reset scroll offset
    fn prepare_preview(&mut self, text: &str) {
        self.preview_text = text.to_string();

        // Detect fenced code block and language
        let (code_lang, code_body) = if let Some(first_line) = text.lines().next() {
            if first_line.starts_with("```") {
                let lang = first_line.trim_start_matches("```").trim();
                let body = text.lines().skip(1).collect::<Vec<_>>().join("\n");
                (Some(lang.to_string()), body)
            } else {
                (None, text.to_string())
            }
        } else {
            (None, text.to_string())
        };

        let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();

        if let Some(lang_token) = code_lang {
            // Syntax highlight using syntect
            let ss: &SyntaxSet = &SYNTAX_SET;
            let theme = &THEME_SET.themes["base16-ocean.dark"];
            let syntax: &SyntaxReference = ss
                .find_syntax_by_token(&lang_token)
                .unwrap_or_else(|| ss.find_syntax_plain_text());
            let mut h = HighlightLines::new(syntax, theme);

            for line in LinesWithEndings::from(&code_body) {
                let ranges = h.highlight_line(line, ss).unwrap_or_default();
                let mut spans = Vec::new();
                for (style, piece) in ranges {
                    let fg = syn_color_to_tui(style.foreground);
                    let mut tui_style = Style::default().fg(fg);
                    if style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::BOLD)
                    {
                        tui_style = tui_style.add_modifier(Modifier::BOLD);
                    }
                    if style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::ITALIC)
                    {
                        tui_style = tui_style.add_modifier(Modifier::ITALIC);
                    }
                    spans.push(Span::styled(piece.to_string(), tui_style));
                }
                lines.push(ratatui::text::Line::from(spans));
            }
        } else {
            // Plain text lines
            for l in code_body.lines() {
                lines.push(ratatui::text::Line::from(l.to_string()));
            }
        }

        self.preview_lines = lines;
        self.preview_offset = 0;
    }

    fn handle_mouse_input(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => match self.mode {
                Mode::Preview => {
                    if self.preview_offset + 1 < self.preview_lines.len() {
                        self.preview_offset += 1;
                    }
                }
                Mode::Normal | Mode::Search => self.next_item(),
            },
            MouseEventKind::ScrollUp => match self.mode {
                Mode::Preview => {
                    self.preview_offset = self.preview_offset.saturating_sub(1);
                }
                Mode::Normal | Mode::Search => self.previous_item(),
            },
            _ => {}
        }
    }

    fn delete_selected_item(&mut self) -> Result<()> {
        let Some(selected) = self.list_state.selected() else {
            return Ok(());
        };
        let Some(item_with_ts) = self.filtered_items.get(selected).cloned() else {
            return Ok(());
        };
        let hash = item_with_ts.item.hash();
        self.vault.delete(hash)?;
        self.load_items()?;
        self.status_message = "Item deleted".into();
        Ok(())
    }
}

/// Global syntax and theme sets, initialized on first use without `lazy_static`.
pub static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
pub static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

fn syn_color_to_tui(c: syntect::highlighting::Color) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}
