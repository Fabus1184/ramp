use std::io::{Stdout, Write};

use crossterm::event::Event;
use ratatui::{
    prelude::{CrosstermBackend, Rect},
    text::Line,
    widgets::Paragraph,
    Frame,
};

use super::Tui;

pub struct Log {
    log: Vec<String>,
}

impl Write for Log {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = String::from_utf8(buf.to_vec())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.log.push(s);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Log {
    pub fn new() -> Self {
        Self { log: vec![] }
    }
}

impl Tui for Log {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) {
        let paragraph = Paragraph::new(
            self.log
                .iter()
                .map(|l| Line::from(l.clone()))
                .collect::<Vec<_>>(),
        );
        f.render_widget(paragraph, area);
    }

    fn input(&mut self, _event: &Event) {}
}
