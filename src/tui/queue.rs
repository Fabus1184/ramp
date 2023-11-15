use std::sync::{Arc, RwLock};

use crossterm::event::Event;
use log::trace;
use ratatui::{
    prelude::Constraint,
    style::{Color, Modifier, Style, Stylize},
    widgets::Table,
};

use crate::{cache::Cache, player::facade::PlayerFacade, tui::song_table};

use super::Tui;

pub struct Queue {
    cache: Arc<Cache>,
    player: Arc<RwLock<PlayerFacade>>,
}

impl Queue {
    pub fn new(cache: Arc<Cache>, player: Arc<RwLock<PlayerFacade>>) -> Self {
        Queue { cache, player }
    }
}

impl Tui for Queue {
    fn draw(&self, area: ratatui::prelude::Rect, f: &mut ratatui::Frame) -> anyhow::Result<()> {
        trace!("drawing queue");

        trace!("lock player");
        let player = self.player.read().unwrap();

        let items = player
            .queue
            .iter()
            .map(|p| self.cache.get(p).unwrap().unwrap().as_file().unwrap())
            .map(|s| song_table::song_row(s))
            .collect::<Vec<_>>();

        let table = Table::new(items.clone())
            .header(song_table::HEADER())
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

        f.render_widget(table, area);

        Ok(())
    }

    fn input(&mut self, _event: &Event) -> anyhow::Result<()> {
        Ok(())
    }
}
