use std::io::Stdout;

use crossterm::event::Event;
use ratatui::{
    prelude::{CrosstermBackend, Rect},
    Frame,
};

use super::Tui;

pub struct Log {}

impl Log {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tui for Log {
    fn draw(&self, _area: Rect, _f: &mut Frame<'_, CrosstermBackend<Stdout>>) {}

    fn input(&mut self, _event: &Event) {}
}
