use std::cmp::Ordering;

use crossterm::event::{Event, KeyCode, KeyEvent};
use itertools::Itertools;
use log::{debug, error};
use ratatui::{
    prelude::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Row, Table, TableState},
};

use crate::{cache::Cache, player::Player};

use super::TuiStateful;

pub struct Files<'a> {
    cache: &'a Cache,
    path: Vec<String>,
    selected: Vec<usize>,
}

impl<'a> Files<'a> {
    pub fn new(cache: &'a Cache) -> Self {
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
        }
    }
}

impl<'a> TuiStateful<&mut Player<'a>> for Files<'a> {
    type W = Table<'a>;

    fn tui(&self, _player: &mut Player<'a>) -> (Table<'a>, TableState) {
        let items = items(self.cache, &self.path);
        let table = Table::new(
            items
                .iter()
                .map(|&(f, c)| {
                    Row::new(match c.as_ref() {
                        Cache::File { song, .. } => [
                            song.track.as_ref(),
                            song.artist.as_ref(),
                            song.title.as_ref(),
                            song.album.as_ref(),
                        ]
                        .map(|s| s.map(|s| s.as_str()).unwrap_or("").to_string()),
                        Cache::Directory { .. } => ["", "", f.as_str(), ""].map(|s| s.to_string()),
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
        let table_state = TableState::default()
            .with_selected(Some((selected).min(items.len() - 1).max(0) as usize))
            .with_offset(selected / 2);

        /*
        format!(" {} ", self.path.iter().fold(PathBuf::new(), |mut acc, s| {
                                acc.push(s);
                                acc
                            })
                            .as_os_str()
                            .to_str()
                            .unwrap_or("Failed to get path")
                            .to_owned()
                    ) */

        (table, table_state)
    }

    fn input(&mut self, event: &crossterm::event::Event, player: &mut Player<'a>) {
        let l = items(self.cache, &self.path).len();

        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
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
                    let item = *items(self.cache, &self.path)
                        .get(selected)
                        .expect("Failed to get selected file");

                    let (f, c) = item;

                    match c.as_ref() {
                        Cache::File { song, .. } => {
                            debug!("playing song: {song:?}");

                            player.queue(song, &self.path, f).unwrap_or_else(|e| {
                                error!("Failed to queue song: {e}");
                            });
                        }
                        Cache::Directory { .. } => {
                            self.path.push(f.to_string());
                            self.selected.push(0);
                        }
                    }
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

fn items<'a>(cache: &'a Cache, path: &Vec<String>) -> Vec<(&'a String, &'a Box<Cache>)> {
    cache
        .get(path)
        .map(|c| match c {
            Cache::File { .. } => panic!("File returned from Cache::get"),
            Cache::Directory { children } => children
                .iter()
                .sorted_by(|&(f1, c1), &(f2, c2)| match (c1.as_ref(), c2.as_ref()) {
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
