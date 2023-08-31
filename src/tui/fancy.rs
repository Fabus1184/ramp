use std::{
    io::Stdout,
    sync::{Arc, Mutex},
};

use crossterm::event::Event;
use image::imageops::FilterType;
use log::trace;
use ratatui::{
    prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Padding, Paragraph, Row, Table},
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
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
        trace!("locking player");
        let player = self.player.lock().expect("Failed to lock player");

        let standard_tags = Table::new(
            player
                .current()
                .map(|(s, _)| {
                    s.standard_tags
                        .iter()
                        .map(|(k, v)| (format!("{:?}", k), v))
                        .chain(s.other_tags.iter().map(|(k, v)| (k.clone(), v)))
                        .map(|(k, v)| {
                            Row::new(vec![Cell::from(k).gray().bold(), Cell::from(v.to_string())])
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        )
        .widths(&[Constraint::Percentage(50), Constraint::Percentage(50)])
        .block(
            Block::new()
                .padding(Padding::new(1, 0, 0, 0))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(
                    " {} ",
                    player
                        .current()
                        .map(|(_, p)| {
                            p.to_str()
                                .ok_or(anyhow::anyhow!("Failed to convert Path to str: {:?}", p))
                        })
                        .unwrap_or(Ok(""))?,
                ))
                .title_style(Style::default().bold().light_blue()),
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

        if let Some(image) = player
            .current_cover()
            .and_then(|x| image::load_from_memory(x).ok())
        {
            let resized = image.resize(
                (right.width as u32 - 1) * 2,
                (right.height as u32 - 1) * 2,
                FilterType::CatmullRom,
            );

            let rgb = resized
                .as_flat_samples_u8()
                .expect("Failed to convert image")
                .samples
                .chunks(3)
                .collect::<Vec<_>>();

            let mut lines = vec![];
            for y in (0..resized.height()).step_by(2) {
                let mut line = vec![];
                for x in 0..resized.width() {
                    let [r1, g1, b1] = rgb
                        .get((y * resized.width() + x) as usize)
                        .and_then(|&x| x.try_into().ok())
                        .unwrap_or([0, 0, 0]);
                    let [r2, g2, b2] = rgb
                        .get((y * resized.width() + x + resized.width()) as usize)
                        .and_then(|&x| x.try_into().ok())
                        .unwrap_or([0, 0, 0]);
                    line.push(
                        Span::from("▀")
                            .fg(Color::Rgb(r1, g1, b1))
                            .bg(Color::Rgb(r2, g2, b2)),
                    );
                }
                lines.push(Line::from(line));
            }

            let image = Paragraph::new(lines).alignment(Alignment::Center).block(
                Block::new()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL)
                    .title(" Album Art ")
                    .title_style(Style::default().light_blue().bold()),
            );

            f.render_widget(image, right);
            f.render_widget(standard_tags, left);
        }

        Ok(())
    }

    fn input(&mut self, _event: &Event) -> anyhow::Result<()> {
        Ok(())
    }
}
