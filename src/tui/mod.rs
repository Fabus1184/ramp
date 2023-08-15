mod files;
mod queue;
mod tabs;

use std::sync::{Arc, Mutex};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use ratatui::{
    backend::CrosstermBackend,
    prelude::{Margin, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, StatefulWidget, Widget},
    Terminal,
};

use crate::{cache::Cache, config::Config, player::Player};

use self::{files::Files, queue::Queue, tabs::Tabs};

trait TuiStateful<T> {
    type W: StatefulWidget;
    fn tui(
        &self,
        t: T,
    ) -> (
        Self::W,
        <<Self as TuiStateful<T>>::W as StatefulWidget>::State,
    );
    fn input(&mut self, event: &Event, t: T);
}

trait Tui<T> {
    type W: Widget;
    fn tui(&self, t: T) -> Self::W;
    fn input(&mut self, event: &Event, t: T);
}

pub fn tui<'a>(
    _config: &Config,
    cache: &'a Cache,
    player: Arc<Mutex<Player<'a>>>,
) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    enable_raw_mode()?;
    terminal.clear()?;

    let mut tabs = Tabs::new();
    let mut files = Files::new(cache);
    let mut queue = Queue::new();

    loop {
        terminal.draw(|f| {
            let border = Margin {
                vertical: 1,
                horizontal: 1,
            };

            let tabs_area = Rect::new(0, 0, f.size().width, 3);
            let inner_area = Rect::new(0, 0, f.size().width, f.size().height).inner(&border);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan));

            f.render_widget(block.clone(), f.size());

            let w = tabs.tui(());
            f.render_widget(w, tabs_area);

            let mut player = player.lock().unwrap();
            match tabs.selected {
                0 => {
                    let (table, mut state) = files.tui(&mut player);
                    f.render_stateful_widget(table, inner_area, &mut state);
                }
                1 => {
                    let table = queue.tui(&mut player);
                    f.render_widget(table, inner_area);
                }
                _ => {}
            }
        })?;

        let event = event::read()?;
        tabs.input(&event, ());

        let mut player = player.lock().unwrap();
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Char(' ') => player.play_pause().expect("Failed to play/pause"),
                KeyCode::Char('n') => player.skip().expect("Failed to skip"),
                KeyCode::Char('s') => player.stop().expect("Failed to stop"),
                KeyCode::Char('c') => player.clear().expect("Failed to clear"),
                KeyCode::Char('q') => break,
                _ => match tabs.selected {
                    0 => {
                        files.input(&event, &mut player);
                    }
                    _ => {}
                },
            },
            _ => {}
        }
    }

    disable_raw_mode()?;
    terminal.show_cursor()?;
    terminal.clear()?;

    Ok(())
}
