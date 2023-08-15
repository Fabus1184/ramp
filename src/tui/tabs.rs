use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::style::{Modifier, Style};

use super::Tui;

pub struct Tabs<'a> {
    pub selected: usize,
    phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Tabs<'a> {
    pub fn new() -> Self {
        Self {
            selected: 0,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<'a> Tui<()> for Tabs<'a> {
    type W = ratatui::widgets::Tabs<'a>;

    fn tui(&self, _t: ()) -> Self::W {
        ratatui::widgets::Tabs::new(vec![" Files 🗃️  ", " Queue 🗂️  "])
            .select(self.selected)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
    }

    fn input(&mut self, event: &Event, _t: ()) {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Tab => {
                    self.selected = (self.selected + 1) % 2;
                }
                _ => {}
            },
            _ => {}
        }
    }
}
