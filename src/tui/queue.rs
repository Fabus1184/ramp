use std::sync::{Arc, Mutex};

use crossterm::event::Event;
use log::trace;
use ratatui::{
    prelude::Constraint,
    style::{Color, Modifier, Style},
    widgets::Table,
};

use crate::{player::Player, tui::song_table};

use super::Tui;

pub struct Queue {
    player: Arc<Mutex<Player>>,
}

impl Queue {
    pub fn new(player: Arc<Mutex<Player>>) -> Self {
        Queue { player }
    }
}

impl Tui for Queue {
    fn draw(
        &self,
        area: ratatui::prelude::Rect,
        f: &mut ratatui::Frame<'_, ratatui::prelude::CrosstermBackend<std::io::Stdout>>,
    ) {
        trace!("drawing queue");

        trace!("lock player");
        let player = self.player.lock().unwrap();

        let items = player
            .nexts()
            .map(|s| song_table::song_row(s))
            .collect::<Vec<_>>();

        let table = Table::new(items.clone())
            .header(song_table::HEADER())
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
