use std::sync::Mutex;

use crossterm::event::Event;
use log::trace;
use ratatui::{
    prelude::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Row, Table},
};

use crate::player::Player;

use super::Tui;

pub struct Queue<'a> {
    player: &'a Mutex<Player<'a>>,
}

impl<'a> Queue<'a> {
    pub fn new(player: &'a Mutex<Player<'a>>) -> Self {
        Queue { player }
    }
}

impl Tui for Queue<'_> {
    fn draw(
        &self,
        area: ratatui::prelude::Rect,
        f: &mut ratatui::Frame<'_, ratatui::prelude::CrosstermBackend<std::io::Stdout>>,
    ) {
        trace!("drawing queue, lock");

        let player = self.player.lock().unwrap();

        let table = Table::new(
            std::iter::once(player.current())
                .flatten()
                .chain(player.nexts())
                .map(|s| {
                    Row::new({
                        [
                            s.track.as_ref(),
                            s.artist.as_ref(),
                            s.title.as_ref(),
                            s.album.as_ref(),
                        ]
                        .map(|s| s.map(|s| s.as_str()).unwrap_or("").to_string())
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

        f.render_widget(table, area);
    }

    fn input(&mut self, _event: &Event) {}
}
