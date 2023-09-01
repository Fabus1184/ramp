use std::{
    cmp::Ordering,
    io::Stdout,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use itertools::Itertools;
use log::trace;
use ratatui::{
    prelude::{Constraint, CrosstermBackend, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Table, TableState},
    Frame,
};

use crate::{
    cache::{Cache, CacheEntry},
    player::Player,
    song::StandardTagKey,
    tui::song_table,
};

use super::Tui;

#[derive(Debug, PartialEq, Eq)]
enum FilterState {
    Disabled,
    Active { input: String, selected: bool },
}

pub struct Files {
    cache: Arc<Cache>,
    path: PathBuf,
    selected: Vec<usize>,
    player: Arc<Mutex<Player>>,
    filter: FilterState,
}

impl Files {
    pub fn new(cache: Arc<Cache>, player: Arc<Mutex<Player>>) -> Self {
        Self {
            path: std::path::Path::new("/")
                .canonicalize()
                .expect("Failed to get directory")
                .components()
                .map(|c| {
                    c.as_os_str()
                        .to_str()
                        .expect("Failed to convert path to string")
                        .to_string()
                })
                .collect(),
            selected: vec![0],
            cache,
            player,
            filter: FilterState::Disabled,
        }
    }

    fn input_files(&mut self, event: &Event) -> anyhow::Result<()> {
        trace!("input_files: {:?}", event);

        let l = self.items()?.count();

        trace!("lock player");
        let mut player = self.player.lock().expect("Failed to lock player");

        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event
        {
            match code {
                KeyCode::Char('f') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter = FilterState::Active {
                        input: String::new(),
                        selected: true,
                    };
                }
                KeyCode::Char(' ') => {
                    player.play_pause().expect("Failed to play/pause");
                }
                KeyCode::Char('n') => player.skip().expect("Failed to skip"),
                KeyCode::Char('s') => player.stop().expect("Failed to stop"),
                KeyCode::Char('c') => player.clear().expect("Failed to clear"),
                KeyCode::Up => {
                    self.selected
                        .last_mut()
                        .map(|i| *i = i.checked_sub(1).unwrap_or(0));
                }
                KeyCode::Down => {
                    self.selected.last_mut().map(|i| *i = (*i + 1).min(l - 1));
                }
                KeyCode::PageUp => {
                    self.selected
                        .last_mut()
                        .map(|i| *i = i.checked_sub(25).unwrap_or(0));
                }
                KeyCode::PageDown => {
                    self.selected.last_mut().map(|i| *i = (*i + 25).min(l - 1));
                }
                KeyCode::End => {
                    self.selected.last_mut().map(|i| *i = l - 1);
                }
                KeyCode::Home => {
                    self.selected.last_mut().map(|i| *i = 0);
                }
                KeyCode::Enter => {
                    let selected = *self.selected.last().expect("Failed to get selected index");
                    let (f, c) = self
                        .items()?
                        .nth(selected)
                        .expect("Failed to get item")
                        .clone();

                    match c {
                        CacheEntry::File { .. } => {
                            trace!("queueing song: {:?}", self.path);
                            player.queue(&self.path.join(f)).expect("Failed to queue");
                        }
                        CacheEntry::Directory { .. } => {
                            self.path.push(f.clone());
                            self.selected.push(0);
                        }
                    }

                    trace!("unlock player");
                }
                KeyCode::Backspace => {
                    if self.path.pop() {
                        self.selected.pop();
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn items<'a>(&'a self) -> anyhow::Result<impl Iterator<Item = (&'a String, &'a CacheEntry)>> {
        self.cache
            .get(&self.path)?
            .ok_or(anyhow::anyhow!("Cache::get {:?} returned None", self.path))
            .and_then(|d| d.as_directory())
            .map(|d| {
                d.iter()
                    .filter(|(f, c)| match &self.filter {
                        FilterState::Disabled => true,
                        FilterState::Active { input, .. } => match c {
                            CacheEntry::File { song } => {
                                song.standard_tags.iter().any(|(_, v)| {
                                    v.to_string().to_lowercase().contains(&input.to_lowercase())
                                }) || f.to_lowercase().contains(&input.to_lowercase())
                            }
                            CacheEntry::Directory { .. } => {
                                f.to_lowercase().contains(&input.to_lowercase())
                            }
                        },
                    })
                    .sorted_by(|(f1, c1), (f2, c2)| match (c1, c2) {
                        (
                            CacheEntry::File { song: song1, .. },
                            CacheEntry::File { song: song2, .. },
                        ) => {
                            let t1 = song1
                                .standard_tags
                                .get(&StandardTagKey::TrackNumber)
                                .map(|v| v.to_string())
                                .and_then(|v| v.parse::<u32>().ok());
                            let t2 = song2
                                .standard_tags
                                .get(&StandardTagKey::TrackNumber)
                                .map(|v| v.to_string())
                                .and_then(|v| v.parse::<u32>().ok());

                            match (t1, t2) {
                                (None, None) => f1.cmp(f2),
                                (None, Some(_)) => Ordering::Less,
                                (Some(_), None) => Ordering::Greater,
                                (Some(a), Some(b)) => a.cmp(&b),
                            }
                        }
                        (CacheEntry::File { .. }, CacheEntry::Directory { .. }) => Ordering::Less,
                        (CacheEntry::Directory { .. }, CacheEntry::File { .. }) => {
                            Ordering::Greater
                        }
                        (CacheEntry::Directory { .. }, CacheEntry::Directory { .. }) => f1.cmp(f2),
                    })
            })
    }
}

impl Tui for Files {
    fn draw(&self, area: Rect, f: &mut Frame<'_, CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
        trace!("drawing files");

        let (inner_area, filter_area) = match self.filter {
            FilterState::Disabled => (area, None),
            FilterState::Active { .. } => {
                let layout = Layout::new()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(area);
                (layout[0], Some(layout[1]))
            }
        };

        let search_bar = Paragraph::new(Line::from(match &self.filter {
            FilterState::Disabled => vec![],
            FilterState::Active {
                input,
                selected: true,
            } => vec![
                Span::from("Filter: ").bold(),
                Span::from(input.clone()).light_yellow(),
                Span::from("_").light_yellow().slow_blink(),
            ],
            FilterState::Active {
                input,
                selected: false,
            } => vec![
                Span::from("Filter: ").bold(),
                Span::from(input.clone()).light_yellow(),
            ],
        }));

        let items = self
            .items()?
            .map(|(f, c)| song_table::cache_row(f, c))
            .collect::<Vec<_>>();

        let len = items.len();

        let table = Table::new(items)
            .header(song_table::HEADER().light_blue().bold())
            .fg(Color::Rgb(210, 210, 210))
            .highlight_style(Style::default().light_yellow().bold())
            .highlight_symbol("⏯️  ")
            .column_spacing(4)
            .widths(&[
                Constraint::Percentage(5),
                Constraint::Percentage(15),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ]);

        let selected = *self.selected.last().expect("Failed to get selected index");
        let mut table_state = TableState::default()
            .with_selected(Some((selected).min(len - 1).max(0) as usize))
            .with_offset({
                if len <= area.height as usize {
                    0
                } else if selected > area.height as usize / 2 {
                    if selected < len + 1 - area.height as usize / 2 {
                        selected - area.height as usize / 2
                    } else {
                        len + 1 - area.height as usize
                    }
                } else {
                    0
                }
            });

        let breadcrums = Paragraph::new(Line::from(
            Span::from(format!("🔗 {}", self.path.display().to_string()))
                .bold()
                .light_red(),
        ));

        let layout = Layout::new()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner_area);

        let (path_area, files_area) = (layout[0], layout[1]);

        f.render_widget(breadcrums, path_area);
        f.render_stateful_widget(table, files_area, &mut table_state);

        if let Some(search_bar_area) = filter_area {
            f.render_widget(search_bar, search_bar_area);
        }

        Ok(())
    }

    fn input(&mut self, event: &Event) -> anyhow::Result<()> {
        trace!("input: {:?}", event);

        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event
        {
            match &mut self.filter {
                FilterState::Disabled => {
                    self.input_files(event)?;
                }
                FilterState::Active { input, selected } => match code {
                    KeyCode::Esc => {
                        self.filter = FilterState::Disabled;
                    }
                    KeyCode::Enter if *selected => {
                        *selected = false;
                    }
                    KeyCode::Char('f')
                        if modifiers.contains(KeyModifiers::CONTROL) && !*selected =>
                    {
                        *selected = true;
                    }
                    KeyCode::Char(c) if *selected => {
                        input.push(*c);
                    }
                    KeyCode::Backspace if *selected => {
                        input.pop();
                    }
                    _ if !*selected => {
                        self.input_files(event)?;
                    }
                    _ => {}
                },
            }
        }

        let l = self.items()?.count();

        self.selected
            .last_mut()
            .filter(|i| **i >= l)
            .map(|i| *i = l - 1);

        Ok(())
    }
}
