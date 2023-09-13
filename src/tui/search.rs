use std::{
    io::Stdout,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{Event, KeyCode, KeyEvent};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use ratatui::{
    prelude::{Constraint, CrosstermBackend, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Table, TableState},
    Frame,
};
use strsim::jaro_winkler;

use crate::{
    cache::{Cache, CacheEntry},
    player::Player,
    song::{Song, StandardTagKey},
};

use super::{song_table, Tui, UNKNOWN_STRING};

pub struct Search {
    keyword: String,
    cache: Arc<Cache>,
    selected: usize,
    player: Arc<Mutex<Player>>,
    items: Vec<(Song, PathBuf)>,
}

impl Search {
    pub fn new(cache: Arc<Cache>, player: Arc<Mutex<Player>>) -> Self {
        Self {
            keyword: String::new(),
            cache,
            selected: 0,
            player,
            items: vec![],
        }
    }

    fn update_items(&mut self) {
        self.items = self
            .cache
            .songs()
            .map(|(s, p)| {
                let l = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase());
                (
                    s,
                    p,
                    OrderedFloat(-jaro_winkler(
                        self.keyword.to_lowercase().as_str(),
                        s.standard_tags
                            .get(&StandardTagKey::TrackTitle)
                            .map(|s| s.to_string().to_lowercase())
                            .or(l)
                            .unwrap_or(UNKNOWN_STRING.to_string())
                            .to_lowercase()
                            .as_str(),
                    )),
                )
            })
            .sorted_unstable_by_key(|&(_, _, x)| x)
            .take_while(|&(_, _, x)| x <= OrderedFloat(0.0))
            .map(|(s, p, _)| (s.clone(), p))
            .collect::<Vec<_>>();
    }
}

impl Tui for Search {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                ratatui::prelude::Constraint::Min(1),
                ratatui::prelude::Constraint::Length(1),
            ])
            .split(area);

        let input = Paragraph::new(Line::from(vec![
            Span::from("Search: ")
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            Span::from(self.keyword.clone()).add_modifier(Modifier::ITALIC),
            Span::from("_").add_modifier(Modifier::SLOW_BLINK),
        ]));

        let table = Table::new(
            self.items
                .iter()
                .map(|(s, p)| {
                    let filename = p
                        .file_name()
                        .ok_or(anyhow::anyhow!("Failed to get filename from path {:?}", p))?
                        .to_str()
                        .ok_or(anyhow::anyhow!("Failed to convert OsString to str {:?}", p))?;
                    Ok(song_table::cache_row(
                        filename,
                        &CacheEntry::File { song: s.clone() },
                    ))
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        )
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

        f.render_stateful_widget(
            table,
            layout[0],
            &mut TableState::default().with_selected(Some(self.selected)),
        );
        f.render_widget(input, layout[1]);

        Ok(())
    }

    fn input(&mut self, event: &Event) -> anyhow::Result<()> {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Char(c) => {
                    self.keyword.push(*c);
                    self.update_items();
                }
                KeyCode::Backspace => {
                    if self.keyword.pop().is_some() {
                        self.update_items();
                    }

                    if self.keyword.len() == 0 {
                        self.items.clear();
                    }
                }
                KeyCode::Esc => {
                    self.keyword.clear();
                    self.items.clear();
                }
                KeyCode::Down => {
                    self.selected += 1;
                }
                KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                KeyCode::Enter => {
                    let (_, path) = self
                        .items
                        .get(self.selected)
                        .ok_or(anyhow::anyhow!("Failed to get selected Song"))?
                        .clone();

                    self.player
                        .lock()
                        .map_err(|e| anyhow::anyhow!("Failed to lock player: {:?}", e))?
                        .queue(path)
                        .expect("Failed to queue song");
                }
                _ => {}
            },
            _ => {}
        }

        self.selected = self.selected.clamp(0, self.items.len());

        Ok(())
    }
}
