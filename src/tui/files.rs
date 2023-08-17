use std::{cmp::Ordering, io::Stdout, sync::Mutex};

use crossterm::event::{Event, KeyCode, KeyEvent};
use itertools::Itertools;
use log::{debug, error, trace};
use ratatui::{
    prelude::{Constraint, CrosstermBackend, Rect},
    style::{Color, Modifier, Style},
    widgets::{Row, Table, TableState},
    Frame,
};

use crate::{cache::Cache, player::Player};

use super::Tui;

pub struct Files<'a> {
    cache: &'a Cache,
    path: Vec<String>,
    selected: Vec<usize>,
    player: &'a Mutex<Player<'a>>,
}

impl<'a> Files<'a> {
    pub fn new(cache: &'a Cache, player: &'a Mutex<Player<'a>>) -> Self {
        Self {
            path: std::path::Path::new("/")
                .canonicalize()
                .expect("Failed to get directory")
                .components()
                .map(|c| {
                    c.as_os_str()
                        .to_str()
                        .expect("Failed to convert path to string")
                        .to_string()
                })
                .collect(),
            selected: vec![0],
            cache,
            player,
        }
    }
}

impl<'a> Tui for Files<'a> {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) {
        trace!("drawing files");

        let items = items(self.cache, &self.path);

        let table = Table::new(
            items
                .iter()
                .map(|(f, c)| {
                    Row::new(match c {
                        Cache::File { ref song, .. } => [
                            song.track.as_ref(),
                            song.artist.as_ref(),
                            song.title.as_ref(),
                            song.album.as_ref(),
                        ]
                        .map(|s| s.map(|s| s.as_str()).unwrap_or("<unknown>").to_string()),
                        Cache::Directory { .. } => {
                            ["-", "-", f.as_str(), "-"].map(|s| s.to_string())
                        }
                    })
                })
                .collect::<Vec<_>>(),
        )
        .header(
            Row::new(vec![
                "Track #️⃣ ",
                "Artist 🧑‍🎤 ",
                "File / Title 🎶 ",
                "Album 🖼️ ",
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .style(Style::default().fg(Color::Rgb(210, 210, 210)))
        .highlight_style(
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("⏯️  ")
        .column_spacing(4)
        .widths(&[
            Constraint::Percentage(5),
            Constraint::Percentage(15),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ]);

        let selected = *self.selected.last().expect("Failed to get selected index");
        let mut table_state = TableState::default()
            .with_selected(Some((selected).min(items.len() - 1).max(0) as usize))
            .with_offset({
                if selected > f.size().height as usize / 2 {
                    if selected < items.len() - f.size().height as usize / 2 {
                        selected - f.size().height as usize / 2
                    } else {
                        items.len() - f.size().height as usize
                    }
                } else {
                    0
                }
            });

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn input(&mut self, event: &Event) {
        trace!("input: {:?}", event);

        let l = items(self.cache, &self.path).len();

        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Char(' ') => self
                    .player
                    .lock()
                    .unwrap()
                    .play_pause()
                    .expect("Failed to play/pause"),
                KeyCode::Char('n') => self.player.lock().unwrap().skip().expect("Failed to skip"),
                KeyCode::Char('s') => self.player.lock().unwrap().stop().expect("Failed to stop"),
                KeyCode::Char('c') => self
                    .player
                    .lock()
                    .unwrap()
                    .clear()
                    .expect("Failed to clear"),
                KeyCode::Up => {
                    self.selected
                        .last_mut()
                        .map(|i| *i = i.checked_sub(1).unwrap_or(0));
                }
                KeyCode::Down => {
                    self.selected.last_mut().map(|i| *i = (*i + 1).min(l - 1));
                }
                KeyCode::PageUp => {
                    self.selected
                        .last_mut()
                        .map(|i| *i = i.checked_sub(25).unwrap_or(0));
                }
                KeyCode::PageDown => {
                    self.selected.last_mut().map(|i| *i = (*i + 25).min(l - 1));
                }
                KeyCode::End => {
                    self.selected
                        .last_mut()
                        .map(|i| *i = items(self.cache, &self.path).len() - 1);
                }
                KeyCode::Home => {
                    self.selected.last_mut().map(|i| *i = 0);
                }
                KeyCode::Enter => {
                    let selected = *self.selected.last().expect("Failed to get selected index");
                    let (f, c) = {
                        let is = items(self.cache, &self.path);
                        *is.get(selected).expect("Failed to get selected file")
                    };

                    match c {
                        Cache::File { ref song, .. } => {
                            debug!("playing song: {song:?}");

                            trace!("queueing song, lock player");
                            self.player
                                .lock()
                                .unwrap()
                                .queue(song, &self.path, &f)
                                .unwrap_or_else(|e| {
                                    error!("Failed to queue song: {e}");
                                });
                        }
                        Cache::Directory { .. } => {
                            self.path.push(f.to_string());
                            self.selected.push(0);
                        }
                    }

                    trace!("unlock player");
                }
                KeyCode::Backspace => {
                    if self.path.len() > 1 {
                        self.path.pop();
                        self.selected.pop();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn items<'a>(cache: &'a Cache, path: &Vec<String>) -> Vec<(&'a String, &'a Cache)> {
    Cache::get(cache, path)
        .map(|c| match c {
            Cache::File { .. } => panic!("File returned from Cache::get"),
            Cache::Directory { ref children } => children
                .iter()
                .sorted_by(|(f1, c1), (f2, c2)| match (c1, c2) {
                    (Cache::File { song: song1, .. }, Cache::File { song: song2, .. }) => {
                        let t1 = song1.track.as_ref().and_then(|x| str::parse::<u32>(x).ok());
                        let t2 = song2.track.as_ref().and_then(|x| str::parse::<u32>(x).ok());

                        match (t1, t2) {
                            (None, None) => f1.cmp(f2),
                            (None, Some(_)) => Ordering::Less,
                            (Some(_), None) => Ordering::Greater,
                            (Some(a), Some(b)) => a.cmp(&b),
                        }
                    }
                    (Cache::File { .. }, Cache::Directory { .. }) => Ordering::Less,
                    (Cache::Directory { .. }, Cache::File { .. }) => Ordering::Greater,
                    (Cache::Directory { .. }, Cache::Directory { .. }) => f1.cmp(f2),
                })
                .collect::<Vec<_>>(),
        })
        .unwrap_or(vec![])
}
