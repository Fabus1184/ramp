use std::io::Stdout;

use crossterm::event::{Event, KeyCode, KeyEvent};
use itertools::Itertools;
use ratatui::{
    prelude::{CrosstermBackend, Rect},
    widgets::{List, ListItem, Paragraph},
    Frame,
};
use strsim::levenshtein;

use crate::cache::Cache;

use super::Tui;

pub struct Search<'a> {
    keyword: String,
    cache: &'a Cache,
}

impl<'a> Search<'a> {
    pub fn new(cache: &'a Cache) -> Self {
        Self {
            keyword: String::new(),
            cache,
        }
    }
}

impl<'a> Tui for Search<'a> {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) {
        let input = Paragraph::new(format!("{}_", self.keyword.clone()));
        let list = List::new(
            self.cache
                .songs()
                .sorted_by_key(|s| {
                    levenshtein(
                        self.keyword.as_str(),
                        s.title.as_ref().map(|s| s.as_str()).unwrap_or(""),
                    )
                })
                .map(|s| ListItem::new(s.title.clone().unwrap_or("".to_string())))
                .take(50)
                .collect_vec(),
        );

        f.render_widget(
            input,
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );
        f.render_widget(
            list,
            Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: area.height - 1,
            },
        );
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
