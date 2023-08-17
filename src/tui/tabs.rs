use std::{io::Stdout, sync::Mutex};

use crossterm::event::{Event, KeyCode, KeyEvent};
use log::trace;
use ratatui::{
    prelude::{CrosstermBackend, Rect},
    style::{Modifier, Style},
    Frame,
};

use super::Tui;

pub struct Tabs<'a> {
    pub selected: usize,
    pub tabs: Vec<(&'static str, Box<dyn Tui + 'a>)>,
    running: &'a Mutex<bool>,
}

impl<'a> Tabs<'a> {
    pub fn new(tabs: Vec<(&'static str, Box<dyn Tui + 'a>)>, running: &'a Mutex<bool>) -> Self {
        Self {
            selected: 0,
            tabs,
            running,
        }
    }
}

impl Tui for Tabs<'_> {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) {
        let tabs = ratatui::widgets::Tabs::new(self.tabs.iter().map(|(title, _)| *title).collect())
            .select(self.selected)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        f.render_widget(tabs, area);

        let (name, inner) = self.tabs.get(self.selected).expect("Tab not found");
        trace!("tabs::draw, inner: {:?}", name);
        inner.draw(
            Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width - 2,
                height: area.height - 2,
            },
            f,
        );
    }

    fn input(&mut self, event: &Event) {
        trace!("Tabs input: {:?}", event);
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Tab => {
                    self.selected = (self.selected + 1) % self.tabs.len();
                }
                KeyCode::BackTab => {
                    self.selected = (self.selected.wrapping_sub(1)) % self.tabs.len();
                }
                KeyCode::Char('q') => {
                    *self.running.lock().unwrap() = false;
                }
                _ => {
                    let content = self.tabs.get_mut(self.selected).expect("Tab not found");
                    content.1.input(event);
                }
            },
            _ => {}
        }
    }
}
