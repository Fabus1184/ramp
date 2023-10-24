mod fancy;
mod files;
mod queue;
mod search;
mod song_table;
mod status;
mod tabs;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use ratatui::{
    backend::CrosstermBackend,
    prelude::{Constraint, Direction, Layout, Rect},
    Frame, Terminal,
};

use crate::{cache::Cache, config::Config, player::Player};

use self::{fancy::Fancy, files::Files, queue::Queue, search::Search, status::Status, tabs::Tabs};

pub const UNKNOWN_STRING: &'static str = "<unknown>";

pub fn format_duration(duration: Duration) -> String {
    let hours = duration.as_secs() / 3600;
    let minutes = (duration.as_secs() % 3600) / 60;
    let seconds = duration.as_secs() % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

pub trait Tui {
    fn draw(&self, area: Rect, f: &mut Frame) -> anyhow::Result<()>;
    fn input(&mut self, event: &Event) -> anyhow::Result<()>;
}

pub fn tui<'a>(
    _config: Arc<Config>,
    cache: Arc<Cache>,
    player: Arc<Mutex<Player>>,
) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    enable_raw_mode()?;
    terminal.clear()?;

    let running = Mutex::new(true);
    let mut tabs = Tabs::new(
        vec![
            (
                " Files 🗃️ ",
                Box::new(Files::new(cache.clone(), player.clone())),
            ),
            ("Queue 🕰️ ", Box::new(Queue::new(player.clone()))),
            (
                "Search 🔎", /* idk, whatever */
                Box::new(Search::new(cache.clone(), player.clone())),
            ),
            ("Fancy stuff ✨ ", Box::new(Fancy::new(player.clone()))),
        ],
        &running,
    );

    let usage = Status::new(player.clone());

    loop {
        terminal.draw(|f| {
            let main_area = Layout::new()
                .constraints([Constraint::Min(1), Constraint::Length(4)])
                .direction(Direction::Vertical)
                .split(f.size());

            tabs.draw(main_area[0], f).expect("Failed to draw tabs");
            usage.draw(main_area[1], f).expect("Failed to draw usage");
        })?;

        if event::poll(Duration::from_secs_f32(0.2))? {
            tabs.input(&event::read()?)?;
        }

        if !*running.lock().unwrap() {
            break;
        }
    }

    disable_raw_mode()?;
    terminal.clear()?;

    Ok(())
}
