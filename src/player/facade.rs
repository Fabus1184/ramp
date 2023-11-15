use std::{
    sync::{atomic::AtomicBool, Arc, RwLock},
    time::Duration,
};

use symphonia::core::meta::{MetadataRevision, StandardVisualKey};

use crate::song::Song;

use super::Player;

#[derive(Default)]
pub enum PlayerStatus {
    PlayingOrPaused {
        song: Song,
        metadata: Option<MetadataRevision>,
        playing_duration: Arc<RwLock<Duration>>,
        paused: Arc<AtomicBool>,
    },
    #[default]
    Stopped,
}

impl PlayerStatus {
    fn from_internal(player: &Player) -> PlayerStatus {
        match &player.status {
            super::InternalPlayerStatus::PlayingOrPaused {
                song,
                metadata,
                playing_duration,
                stream_paused,
                ..
            } => PlayerStatus::PlayingOrPaused {
                song: song.clone(),
                metadata: metadata.clone(),
                playing_duration: playing_duration.clone(),
                paused: stream_paused.clone(),
            },
            super::InternalPlayerStatus::Stopped => PlayerStatus::Stopped,
        }
    }
}

#[derive(Default)]
pub struct PlayerFacade {
    pub status: PlayerStatus,
    pub queue: Box<[Box<std::path::Path>]>,
}

impl PlayerFacade {
    pub(super) fn from_player(player: &Player) -> PlayerFacade {
        PlayerFacade {
            status: PlayerStatus::from_internal(player),
            queue: player.queue.clone().into_iter().collect(),
        }
    }

    pub fn current_song(&self) -> Option<&Song> {
        match &self.status {
            PlayerStatus::PlayingOrPaused { song, .. } => Some(song),
            _ => None,
        }
    }

    pub fn playing_duration(&self) -> Option<std::time::Duration> {
        match &self.status {
            PlayerStatus::PlayingOrPaused {
                playing_duration, ..
            } => Some(*playing_duration.read().unwrap()),
            _ => None,
        }
    }

    pub fn current_cover(&self) -> Option<&[u8]> {
        match &self.status {
            PlayerStatus::PlayingOrPaused { metadata, .. } => metadata.as_ref(),
            PlayerStatus::Stopped => None,
        }
        .and_then(|m| {
            m.visuals()
                .iter()
                .find(|v| v.usage == Some(StandardVisualKey::FrontCover))
        })
        .map(|v| v.data.as_ref())
    }
}
