use std::{
    io::Stdout,
    sync::{Arc, Mutex},
};

use crossterm::event::Event;
use image::imageops::FilterType;
use log::trace;
use ratatui::{
    prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::player::Player;

use super::Tui;

pub struct Fancy {
    player: Arc<Mutex<Player>>,
}

impl Fancy {
    pub fn new(player: Arc<Mutex<Player>>) -> Self {
        Self { player }
    }
}

impl Tui for Fancy {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) {
        trace!("locking player");
        let player = self.player.lock().expect("Failed to lock player");

        let standard_tags = Paragraph::new(
            player
                .current()
                .map(|s| {
                    s.standard_tags
                        .iter()
                        .map(|(k, v)| {
                            Line::from(vec![
                                Span::from(format!(" {:?}: ", k)),
                                Span::styled(format!("{}", v), Style::default().fg(Color::Gray)),
                            ])
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or(vec![]),
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Standard Tags "),
        );

        let other_tags = Paragraph::new(
            player
                .current()
                .map(|s| {
                    s.other_tags
                        .iter()
                        .map(|(k, v)| {
                            Line::from(vec![
                                Span::from(format!(" {}: ", k)),
                                Span::styled(format!("{}", v), Style::default().fg(Color::Gray)),
                            ])
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or(vec![]),
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Other Tags "),
        );

        let layout = Layout::new()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Length(1),
                Constraint::Percentage(50),
            ])
            .split(area);

        let (left, _seperator, right) = (layout[0], layout[1], layout[2]);
        let layout = Layout::new()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(left);
        let (top, bottom) = (layout[0], layout[1]);

        if let Some(image) = player
            .current()
            .and_then(|x| x.front_cover())
            .and_then(|x| image::load_from_memory(&x.data).ok())
        {
            let resized = image.resize(
                (right.width as u32 - 1) * 2,
                (right.height as u32 - 1) * 2,
                FilterType::CatmullRom,
            );

            let rgb = resized
                .as_flat_samples_u8()
                .unwrap()
                .samples
                .chunks(3)
                .collect::<Vec<_>>();

            let mut lines = vec![];
            for y in (0..resized.height()).step_by(2) {
                let mut line = vec![];
                for x in 0..resized.width() {
                    let [r1, g1, b1] = rgb[(y * resized.width() + x) as usize] else { panic!("Failed to get pixel as RGB") };
                    let [r2, g2, b2] = rgb[(y * resized.width() + x + resized.width()) as usize] else { panic!("Failed to get pixel as RGB") };
                    line.push(Span::styled(
                        "▀",
                        Style::default()
                            .fg(Color::Rgb(*r1, *g1, *b1))
                            .bg(Color::Rgb(*r2, *g2, *b2)),
                    ));
                }
                lines.push(Line::from(line));
            }

            let image = Paragraph::new(lines).alignment(Alignment::Center).block(
                Block::new()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL)
                    .title(" Album Art "),
            );

            f.render_widget(image, right);
            f.render_widget(standard_tags, top);
            f.render_widget(other_tags, bottom);
        }
    }

    fn input(&mut self, _event: &Event) {}
}
