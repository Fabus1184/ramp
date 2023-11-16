use std::sync::{atomic::AtomicBool, Arc};

use crossterm::event::{Event, KeyCode, KeyEvent};
use log::trace;
use ratatui::{
    prelude::{Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{block::Title, Block, BorderType, Borders},
    Frame,
};

use super::Tui;

pub struct Tabs<'a> {
    pub selected: usize,
    pub tabs: Vec<(&'static str, Box<dyn Tui + 'a>)>,
    running: Arc<AtomicBool>,
}

impl<'a> Tabs<'a> {
    pub fn new(tabs: Vec<(&'static str, Box<dyn Tui + 'a>)>, running: Arc<AtomicBool>) -> Self {
        Self {
            selected: 0,
            tabs,
            running,
        }
    }
}

impl Tui for Tabs<'_> {
    fn draw(&self, area: Rect, f: &mut Frame) -> anyhow::Result<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Title::from(Line::from(
                self.tabs
                    .iter()
                    .enumerate()
                    .map(|(i, (name, _))| {
                        Span::styled(
                            *name,
                            if i == self.selected {
                                Style::default()
                                    .add_modifier(Modifier::BOLD)
                                    .fg(Color::LightGreen)
                            } else {
                                Style::default().add_modifier(Modifier::BOLD)
                            },
                        )
                    })
                    .fold(Vec::new(), |mut acc, span| {
                        if acc.is_empty() {
                            acc.push(span);
                        } else {
                            acc.push(Span::from(" | "));
                            acc.push(span);
                        }
                        acc
                    }),
            )));
        f.render_widget(block, area);

        let (name, inner) = self.tabs.get(self.selected).expect("Tab not found");

        trace!("tabs::draw, inner: {:?}", name);
        inner.draw(
            area.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            }),
            f,
        )?;

        Ok(())
    }

    fn input(&mut self, event: &Event) -> anyhow::Result<()> {
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
                    self.running
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                }
                _ => {
                    let content = self.tabs.get_mut(self.selected).expect("Tab not found");
                    content.1.input(event)?;
                }
            },
            _ => {}
        }

        Ok(())
    }
}
