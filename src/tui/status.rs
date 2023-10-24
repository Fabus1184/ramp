use std::{
    sync::{Arc, Mutex},
};

use itertools::Itertools;
use log::trace;
use ratatui::{
    prelude::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{LineGauge, Paragraph},
    Frame,
};

use crate::{player::Player, song::StandardTagKey, tui::format_duration};

use super::{Tui, UNKNOWN_STRING};

pub struct Status {
    player: Arc<Mutex<Player>>,
}

impl Status {
    pub fn new(player: Arc<Mutex<Player>>) -> Self {
        Self { player }
    }
}

impl Tui for Status {
    fn draw(&self, area: Rect, f: &mut Frame) -> anyhow::Result<()> {
        let layout = Layout::default()
            .direction(ratatui::prelude::Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
            .split(area.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            }));

        trace!("locking player");
        let playing = Paragraph::new(
            if let Some((song, path)) = self.player.lock().unwrap().current() {
                let title = song
                    .standard_tags
                    .get(&StandardTagKey::TrackTitle)
                    .map(|s| s.to_string())
                    .or(path
                        .components()
                        .last()
                        .map(|s| s.as_os_str().to_string_lossy().to_string()))
                    .unwrap_or(UNKNOWN_STRING.to_string());

                let artist = song
                    .standard_tags
                    .get(&StandardTagKey::Artist)
                    .map(|s| s.to_string());

                let mut elems = vec![Span::from(" ")];

                if let Some(artist) = artist {
                    elems.push(
                        Span::from(artist)
                            .fg(Color::LightYellow)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    );
                    elems.push(Span::from(" - ").fg(Color::White));
                }

                elems.extend([
                    Span::from(title)
                        .fg(Color::LightYellow)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                    Span::from(format!(" ({})", format_duration(song.duration)))
                        .fg(Color::LightGreen),
                    Span::from(" "),
                ]);

                Line::from(elems)
            } else {
                Line::from(vec![
                    Span::from(" - ").add_modifier(ratatui::style::Modifier::BOLD)
                ])
            },
        )
        .alignment(ratatui::prelude::Alignment::Center);

        trace!("locking player");
        let player = self.player.lock().unwrap();
        let ratio = if let (Some((song, _)), Some(current_time)) =
            (player.current(), player.current_time())
        {
            current_time.as_secs_f64() / song.duration.as_secs_f64()
        } else {
            0.0
        }
        .clamp(0.0, 1.0);

        let progress = LineGauge::default()
            .ratio(ratio as f64)
            .line_set(ratatui::symbols::line::DOUBLE)
            .label("")
            .gauge_style(Style::default().fg(Color::LightBlue).bg(Color::DarkGray));
        let elapsed = format_duration(
            *player
                .current_time()
                .unwrap_or(&std::time::Duration::from_secs(0)),
        );
        let duration = format!(
            " -{}",
            format_duration(
                if let (Some((current, _)), Some(current_time)) =
                    (player.current(), player.current_time())
                {
                    current.duration.saturating_sub(*current_time)
                } else {
                    std::time::Duration::from_secs(0)
                },
            )
        );
        let progress_layout = Layout::new()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(elapsed.len() as u16),
                Constraint::Min(0),
                Constraint::Length(duration.len() as u16),
            ])
            .split(layout[0].inner(&Margin {
                vertical: 0,
                horizontal: 1,
            }));

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

        f.render_widget(Paragraph::new(Line::from(elapsed)), progress_layout[0]);
        f.render_widget(progress, progress_layout[1]);
        f.render_widget(playing, progress_layout[1]);
        f.render_widget(Paragraph::new(Line::from(duration)), progress_layout[2]);

        f.render_widget(usage, layout[1]);

        let block = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded);

        f.render_widget(block, area);

        Ok(())
    }

    fn input(&mut self, _event: &crossterm::event::Event) -> anyhow::Result<()> {
        Ok(())
    }
}
