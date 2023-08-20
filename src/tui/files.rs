use std::{
    cmp::Ordering,
    io::Stdout,
    sync::{Arc, Mutex},
};

use crossterm::event::{Event, KeyCode, KeyEvent};
use itertools::Itertools;
use log::trace;
use ratatui::{
    prelude::{Constraint, CrosstermBackend, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Table, TableState},
    Frame,
};

use crate::{cache::Cache, player::Player, song::StandardTagKey, tui::song_table};

use super::Tui;

pub struct Files {
    cache: Arc<Cache>,
    path: Vec<String>,
    selected: Vec<usize>,
    player: Arc<Mutex<Player>>,
}

impl Files {
    pub fn new(cache: Arc<Cache>, player: Arc<Mutex<Player>>) -> Self {
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

impl Tui for Files {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) {
        trace!("drawing files");

        let items = items(&self.cache.clone(), &self.path)
            .map(|(f, c)| song_table::cache_row(f, c))
            .collect::<Vec<_>>();

        let len = items.len();

        let table = Table::new(items)
            .header(
                song_table::HEADER()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            )
            .fg(Color::Rgb(210, 210, 210))
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
            .with_selected(Some((selected).min(len - 1).max(0) as usize))
            .with_offset({
                if selected > f.size().height as usize / 2 {
                    if selected < len - f.size().height as usize / 2 {
                        selected - f.size().height as usize / 2
                    } else {
                        len - f.size().height as usize
                    }
                } else {
                    0
                }
            });

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn input(&mut self, event: &Event) {
        trace!("input: {:?}", event);

        let l = items(&self.cache.clone(), &self.path).count();

        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Char(' ') => {
                    Player::play_pause(self.player.clone()).expect("Failed to play/pause")
                }
                KeyCode::Char('n') => Player::skip(self.player.clone()).expect("Failed to skip"),
                KeyCode::Char('s') => Player::stop(self.player.clone()).expect("Failed to stop"),
                KeyCode::Char('c') => Player::clear(self.player.clone()).expect("Failed to clear"),
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
                        .map(|i| *i = items(&self.cache.clone(), &self.path).count() - 1);
                }
                KeyCode::Home => {
                    self.selected.last_mut().map(|i| *i = 0);
                }
                KeyCode::Enter => {
                    let selected = *self.selected.last().expect("Failed to get selected index");
                    let cache = self.cache.clone();
                    let mut items = items(&cache, &self.path);
                    let (f, c) = { items.nth(selected).expect("Failed to get selected file") };

                    match c {
                        Cache::File { ref song, .. } => {
                            trace!("queueing song");
                            Player::queue(self.player.clone(), song.clone(), &self.path, &f)
                                .expect("Failed to queue");
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

fn items<'a>(
    cache: &'a Cache,
    path: &Vec<String>,
) -> impl Iterator<Item = (&'a String, &'a Cache)> {
    match cache.get(path).expect("Failed to get cache") {
        Cache::File { .. } => panic!("File returned from Cache::get"),
        Cache::Directory { ref children } => {
            children
                .iter()
                .sorted_by(|(f1, c1), (f2, c2)| match (c1, c2) {
                    (Cache::File { song: song1, .. }, Cache::File { song: song2, .. }) => {
                        let t1 = song1
                            .standard_tags
                            .get(&StandardTagKey::TrackNumber)
                            .map(|v| v.to_string())
                            .and_then(|v| v.parse::<u32>().ok());
                        let t2 = song2
                            .standard_tags
                            .get(&StandardTagKey::TrackNumber)
                            .map(|v| v.to_string())
                            .and_then(|v| v.parse::<u32>().ok());

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
        }
    }
}
