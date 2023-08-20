use std::{
    io::Stdout,
    sync::{Arc, Mutex},
};

use itertools::Itertools;
use ratatui::{
    prelude::{Constraint, CrosstermBackend, Layout, Margin, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{Gauge, Paragraph},
    Frame,
};

use crate::{player::Player, song::StandardTagKey, UNKNOWN_STRING};

use super::Tui;

pub struct Status {
    player: Arc<Mutex<Player>>,
}

impl Status {
    pub fn new(player: Arc<Mutex<Player>>) -> Self {
        Self { player }
    }
}

impl Tui for Status {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) {
        let layout = Layout::default()
            .direction(ratatui::prelude::Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
            .split(area.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            }));

        let playing = Paragraph::new(if let Some(song) = self.player.lock().unwrap().current() {
            let title = song
                .standard_tags
                .get(&StandardTagKey::TrackTitle)
                .map(|s| s.to_string())
                .unwrap_or(UNKNOWN_STRING.to_string());
            let artist = song
                .standard_tags
                .get(&StandardTagKey::Artist)
                .map(|s| s.to_string())
                .unwrap_or(UNKNOWN_STRING.to_string());

            Line::from(vec![
                Span::from(title).add_modifier(ratatui::style::Modifier::BOLD),
                Span::from(" by "),
                Span::from(artist).add_modifier(ratatui::style::Modifier::BOLD),
                Span::from(format!(
                    " ({:01.0}:{:02.0})",
                    (song.duration / 60.0).floor(),
                    song.duration % 60.0
                )),
            ])
        } else {
            Line::from(vec![
                Span::from("Nothing playing").add_modifier(ratatui::style::Modifier::BOLD)
            ])
        })
        .alignment(ratatui::prelude::Alignment::Center);

        let player = self.player.lock().unwrap();
        let percent = if let Some(song) = player.current() {
            (100.0 * player.current_time().unwrap().as_secs_f32() / song.duration).floor() as u16
        } else {
            0
        }
        .clamp(0, 100);

        let progress = Gauge::default()
            .percent(percent)
            .gauge_style(
                ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Green)
                    .bg(ratatui::style::Color::Black),
            )
            .label(Span::from(""))
            .use_unicode(true);

        let usage = Paragraph::new(Text::from(vec![Line::from(
            vec![
                Span::from("⏯️  Space"),
                Span::from("⏭️  n"),
                Span::from("⏹️  s"),
                Span::from("⛔ q"),
            ]
            .into_iter()
            .interleave_shortest(std::iter::repeat(Span::from(" - ")))
            .collect::<Vec<_>>(),
        )
        .alignment(ratatui::prelude::Alignment::Center)]));

        f.render_widget(progress, layout[0]);
        f.render_widget(playing, layout[0]);

        f.render_widget(usage, layout[1]);

        let block = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded);

        f.render_widget(block, area);
    }

    fn input(&mut self, _event: &crossterm::event::Event) {}
}
