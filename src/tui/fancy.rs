use std::{io::Stdout, sync::Mutex};

use crossterm::event::Event;
use image::imageops::FilterType;
use ratatui::{
    prelude::{Alignment, CrosstermBackend, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Padding, Paragraph},
    Frame,
};

use crate::{player::Player, Song, UNKNOWN_STRING};

use super::Tui;

pub struct Fancy<'a> {
    player: &'a Mutex<Player<'a>>,
}

impl<'a> Fancy<'a> {
    pub fn new(player: &'a Mutex<Player<'a>>) -> Self {
        Self { player }
    }
}

impl Tui for Fancy<'_> {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) {
        let player = self.player.lock().expect("Failed to lock player");
        let text = Paragraph::new(
            if let Some((Song { title, artist, .. }, _)) = player.current() {
                Line::from(vec![
                    Span::from("Now playing: "),
                    Span::from(title.as_ref().map(|s| s.as_str()).unwrap_or(UNKNOWN_STRING))
                        .add_modifier(Modifier::BOLD),
                    Span::from(" by "),
                    Span::from(
                        artist
                            .as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or(UNKNOWN_STRING),
                    )
                    .add_modifier(Modifier::BOLD),
                ])
            } else {
                Line::from(vec![
                    Span::from("Nothing playing").add_modifier(Modifier::BOLD)
                ])
            },
        )
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);

        if let Some(image) = player
            .current()
            .and_then(|(_, c)| c)
            .map(|(x, _)| x)
            .and_then(|x| image::load_from_memory(x).ok())
        {
            let resized = image.resize(
                area.width as u32,
                area.height as u32 - 1,
                FilterType::Gaussian,
            );

            let rgb = resized
                .as_flat_samples_u8()
                .unwrap()
                .samples
                .chunks(3)
                .collect::<Vec<_>>();

            let mut lines = vec![];
            for y in 0..resized.height() {
                let mut line = vec![];
                for x in 0..resized.width() {
                    let [r, g, b] = rgb[(y * resized.width() + x) as usize] else { panic!("Failed to get pixel as RGB") };
                    line.push(Span::styled(
                        "██",
                        Style::default().fg(Color::Rgb(*r, *g, *b)),
                    ));
                }
                lines.push(Line::from(line));
            }

            let block = Paragraph::new(lines).alignment(Alignment::Center).block(
                Block::new()
                    .border_type(BorderType::Plain)
                    .border_style(Style::default().fg(Color::White))
                    .padding(Padding::new(1, 1, 1, 1)),
            );

            f.render_widget(
                block,
                Rect {
                    x: area.x,
                    y: area.y + 1,
                    width: area.width,
                    height: area.height - 1,
                },
            );
        }

        f.render_widget(
            text,
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );
    }

    fn input(&mut self, _event: &Event) {}
}
