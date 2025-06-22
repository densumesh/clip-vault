use crate::tui::App;
use clip_vault_core::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub fn run_tui(app: &mut App) -> Result<()> {
    // Setup terminal
    enable_raw_mode().map_err(clip_vault_core::Error::Io)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(clip_vault_core::Error::Io)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(clip_vault_core::Error::Io)?;

    // Run the app
    let res = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode().map_err(clip_vault_core::Error::Io)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(clip_vault_core::Error::Io)?;
    terminal.show_cursor().map_err(clip_vault_core::Error::Io)?;

    res
}
