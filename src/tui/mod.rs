mod files;
mod log;
mod queue;
mod search;
mod tabs;

use std::{io::Stdout, sync::Mutex};

use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use ratatui::{
    backend::CrosstermBackend,
    prelude::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders},
    Frame, Terminal,
};

use crate::{cache::Cache, config::Config, player::Player};

use self::{files::Files, log::Log, queue::Queue, search::Search, tabs::Tabs};

pub trait Tui {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>);
    fn input(&mut self, event: &Event);
}

pub fn tui<'a>(
    _config: &'a Config,
    cache: &'a Cache,
    player: &'a Mutex<Player<'a>>,
) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    enable_raw_mode()?;
    terminal.clear()?;

    let running = Mutex::new(true);
    let mut tabs = Tabs::new(
        vec![
            (" Files 🗃️  ", Box::new(Files::new(cache, player))),
            (" Queue 🕰️  ", Box::new(Queue::new(player))),
            (" Search 🔎  ", Box::new(Search::new(cache))),
            (" Log 📃  ", Box::new(Log::new())),
        ],
        &running,
    );

    loop {
        terminal.draw(|f| {
            f.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan)),
                f.size(),
            );
            tabs.draw(f.size(), f);
        })?;

        tabs.input(&event::read()?);

        if !*running.lock().unwrap() {
            break;
        }
    }

    disable_raw_mode()?;
    terminal.clear()?;

    Ok(())
}
