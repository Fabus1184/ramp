use std::{cmp::Ordering, path::PathBuf};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use itertools::Itertools;
use log::{debug, error, trace};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Gauge, Row, Table, TableState},
    Terminal,
};

use crate::{cache::Cache, config::Config, player::Player};

fn list_select_bounded(list_state: &mut TableState, len: usize, delta: isize) {
    list_state.select(list_state.selected().map(|i| {
        if delta < 0 && -delta > i as isize {
            0
        } else if delta > 0 && delta as usize + i >= len {
            len - 1
        } else {
            (i as isize + delta) as usize
        }
    }));
}

pub fn tui<'a>(config: &Config, cache: &'a Cache, mut player: Player<'a>) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    enable_raw_mode()?;

    let mut table_state = TableState::default();
    table_state.select(Some(0));

    let mut path: Vec<String> = std::path::Path::new("/")
        .canonicalize()
        .expect("Failed to get directory")
        .components()
        .map(|c| {
            c.as_os_str()
                .to_str()
                .expect("Failed to convert path to string")
                .to_string()
        })
        .collect();
    trace!("Path: {:?}", path);

    let items = |path: &Vec<String>| {
        cache
            .get(path)
            .map(|c| match c {
                Cache::File { .. } => panic!("File returned from Cache::get"),
                Cache::Directory { children } => children
                    .iter()
                    .sorted_by(|&(f1, c1), &(f2, c2)| match (c1.as_ref(), c2.as_ref()) {
                        (Cache::File { song: song1, .. }, Cache::File { song: song2, .. }) => {
                            let t1 = song1.track.as_ref().and_then(|x| str::parse::<u32>(x).ok());
                            let t2 = song2.track.as_ref().and_then(|x| str::parse::<u32>(x).ok());

                            match (t1, t2) {
                                (None, None) => f1.cmp(f2),
                                (None, Some(_)) => Ordering::Less,
                                (Some(_), None) => Ordering::Greater,
                                (Some(a), Some(b)) => a.cmp(&b),
                            }
                        }
                        (Cache::File { .. }, Cache::Directory { .. }) => Ordering::Less,
                        (Cache::Directory { .. }, Cache::File { .. }) => Ordering::Greater,
                        (Cache::Directory { .. }, Cache::Directory { .. }) => f1.cmp(f2),
                    })
                    .collect::<Vec<_>>(),
            })
            .unwrap_or(vec![])
    };

    loop {
        let is = items(&path);
        let len = is.len();
        let table = Table::new(
            is.iter()
                .map(|&(f, c)| {
                    Row::new(match c.as_ref() {
                        Cache::File { song, .. } => [
                            song.track.as_ref(),
                            song.artist.as_ref(),
                            song.title.as_ref(),
                            song.album.as_ref(),
                        ]
                        .map(|s| s.map(|s| s.as_str()).unwrap_or("").to_string()),
                        Cache::Directory { .. } => ["", "", f.as_str(), ""].map(|s| s.to_string()),
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Files 🗃️ : {} ",
                    path.iter()
                        .fold(PathBuf::new(), |mut acc, s| {
                            acc.push(s);
                            acc
                        })
                        .as_os_str()
                        .to_str()
                        .unwrap_or("Failed to get path")
                ))
                .border_type(BorderType::Rounded)
                .title_alignment(Alignment::Left)
                .border_style(Style::default().fg(Color::Cyan)),
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

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Green)),
            )
            .style(Style::default())
            .gauge_style(Style::default().fg(Color::Green))
            .label(
                player
                    .current()
                    .map(|s| s.title.as_ref().map(|s| s.as_str()).unwrap_or("No Title"))
                    .unwrap_or("-"),
            )
            .percent(50);

        terminal.draw(|f| {
            let size = f.size();
            let bottom_rect = Rect::new(0, size.height - 3, size.width, 3);
            let table_rect = Rect::new(0, 0, size.width, size.height - 3);
            f.render_stateful_widget(table, table_rect, &mut table_state);
            f.render_widget(gauge, bottom_rect);
        })?;

        match event::read()? {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Char(' ') => player.play_pause().expect("Failed to play/pause"),
                KeyCode::Char('n') => player.skip().expect("Failed to skip"),
                KeyCode::Char('s') => player.stop(),
                KeyCode::Char('c') => player.clear(),
                KeyCode::Char('q') => break,
                KeyCode::Up => list_select_bounded(&mut table_state, len, -1),
                KeyCode::Down => list_select_bounded(&mut table_state, len, 1),
                KeyCode::PageUp => list_select_bounded(&mut table_state, len, -25),
                KeyCode::PageDown => list_select_bounded(&mut table_state, len, 25),
                KeyCode::End => table_state.select(Some(len - 1)),
                KeyCode::Home => table_state.select(Some(0)),
                KeyCode::Enter => table_state
                    .selected()
                    .map(|i| {
                        let &(f, c) = is.get(i).expect("Failed to get selected file");

                        match c.as_ref() {
                            Cache::File { song, .. } => {
                                debug!("playing song: {song:?}");

                                player.queue(song, &path, f).unwrap_or_else(|e| {
                                    error!("Failed to queue song: {e}");
                                });
                            }
                            Cache::Directory { .. } => {
                                path.push(f.to_string());
                                table_state.select(Some(0));
                            }
                        }
                    })
                    .unwrap_or_default(),
                KeyCode::Backspace => {
                    path.pop();
                    table_state.select(Some(0));
                }
                _ => {}
            },
            _ => {}
        }
    }

    disable_raw_mode()?;
    terminal.show_cursor()?;
    terminal.clear()?;

    Ok(())
}
