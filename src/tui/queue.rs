use crossterm::event::Event;
use ratatui::{
    prelude::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Row, Table},
};

use crate::player::Player;

use super::Tui;

pub struct Queue {}

impl Queue {
    pub fn new() -> Self {
        Queue {}
    }
}

impl<'a> Tui<&mut Player<'a>> for Queue {
    type W = Table<'a>;

    fn tui(&self, player: &mut Player<'a>) -> Table<'a> {
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

        table
    }

    fn input(&mut self, _event: &Event, _player: &mut Player<'a>) {}
}
