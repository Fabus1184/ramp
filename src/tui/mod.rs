mod fancy;
mod files;
mod log;
mod queue;
mod search;
mod song_table;
mod status;
mod tabs;

use std::{
    io::Stdout,
    sync::{Arc, Mutex},
    time::Duration,
};

use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use ::log::trace;
use ratatui::{
    backend::CrosstermBackend,
    prelude::{Constraint, Direction, Layout, Rect},
    Frame, Terminal,
};

use crate::{cache::Cache, config::Config, player::Player};

use self::{fancy::Fancy, files::Files, queue::Queue, search::Search, status::Status, tabs::Tabs};

pub trait Tui {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>);
    fn input(&mut self, event: &Event);
}

pub fn tui<'a>(
    _config: &'a Config,
    cache: Arc<Cache>,
    player: Arc<Mutex<Player>>,
) -> std::io::Result<()> {
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
                Box::new(Search::new(cache.clone())),
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

            tabs.draw(main_area[0], f);
            usage.draw(main_area[1], f);
        })?;

        if event::poll(Duration::from_secs_f32(0.2))? {
            tabs.input(&event::read()?);
        }

        trace!("locking player");
        if !*running.lock().unwrap() {
            break;
        }
    }

    disable_raw_mode()?;
    terminal.clear()?;

    Ok(())
}
