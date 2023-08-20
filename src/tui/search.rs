use std::{io::Stdout, sync::Arc};

use crossterm::event::{Event, KeyCode, KeyEvent};
use float_ord::FloatOrd;
use itertools::Itertools;
use ratatui::{
    prelude::{Constraint, CrosstermBackend, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Table},
    Frame,
};
use strsim::jaro_winkler;

use crate::{cache::Cache, song::StandardTagKey, UNKNOWN_STRING};

use super::{song_table, Tui};

pub struct Search {
    keyword: String,
    cache: Arc<Cache>,
}

impl Search {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self {
            keyword: String::new(),
            cache,
        }
    }
}

impl Tui for Search {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) {
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

        let items = self
            .cache
            .songs()
            .sorted_by_key(|s| {
                FloatOrd(-jaro_winkler(
                    self.keyword.as_str(),
                    s.standard_tags
                        .get(&StandardTagKey::TrackTitle)
                        .map(|s| s.to_string())
                        .unwrap_or(UNKNOWN_STRING.to_string())
                        .as_str(),
                ))
            })
            .take(area.height as usize)
            .map(|s| song_table::song_row(s))
            .collect_vec();

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

        f.render_widget(input, layout[1]);
        f.render_widget(table, layout[0]);
    }

    fn input(&mut self, event: &Event) {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Char(c) => {
                    self.keyword.push(*c);
                }
                KeyCode::Backspace => {
                    self.keyword.pop();
                }
                _ => {}
            },
            _ => {}
        }
    }
}
