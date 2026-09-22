use std::time::Duration;
use crossbeam_channel::Receiver;
use mpris_server::{LoopStatus, Metadata, PlaybackStatus, Player, Time, TrackId};
use mpui_config::types::RepeatMode;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub enum MprisAction {
    PlayPause,
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    SeekRelative(i64),
    SetPosition(Duration),
    SetVolume(f64),
    SetLoopStatus(LoopStatus),
    SetShuffle(bool),
}

#[derive(Debug, Clone)]
pub enum MprisUpdate {
    Track {
        id: mpui_core::track::TrackId,
        title: String,
        artist: String,
        album: String,
        duration: Duration,
    },
    Status(PlaybackStatus),
    Position(Duration),
    Seeked(Duration),
    Volume(f64),
    Repeat(RepeatMode),
    Shuffle(bool),
}

pub struct MprisServer {
    pub action_rx: Receiver<MprisAction>,
    pub update_tx: UnboundedSender<MprisUpdate>,
}

impl MprisServer {
    pub fn start() -> Option<Self> {
        let (action_tx, action_rx) = crossbeam_channel::unbounded();
        let (update_tx, mut update_rx): (UnboundedSender<MprisUpdate>, UnboundedReceiver<MprisUpdate>) =
            tokio::sync::mpsc::unbounded_channel();

        std::thread::Builder::new()
            .name("mpui-mpris".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::debug!("Failed to build Tokio runtime for MPRIS: {e}");
                        return;
                    }
                };

                rt.block_on(async move {
                    let local = tokio::task::LocalSet::new();
                    local
                        .run_until(async move {
                            let player = match Player::builder("mpui")
                                .can_play(true)
                                .can_pause(true)
                                .can_go_next(true)
                                .can_go_previous(true)
                                .can_seek(true)
                                .can_control(true)
                                .build()
                                .await
                            {
                                Ok(p) => p,
                                Err(e) => {
                                    tracing::debug!("MPRIS D-Bus service unavailable: {e}");
                                    return;
                                }
                            };

                            let tx = action_tx.clone();
                            player.connect_play_pause(move |_| {
                                let _ = tx.send(MprisAction::PlayPause);
                            });

                            let tx = action_tx.clone();
                            player.connect_play(move |_| {
                                let _ = tx.send(MprisAction::Play);
                            });

                            let tx = action_tx.clone();
                            player.connect_pause(move |_| {
                                let _ = tx.send(MprisAction::Pause);
                            });

                            let tx = action_tx.clone();
                            player.connect_stop(move |_| {
                                let _ = tx.send(MprisAction::Stop);
                            });

                            let tx = action_tx.clone();
                            player.connect_next(move |_| {
                                let _ = tx.send(MprisAction::Next);
                            });

                            let tx = action_tx.clone();
                            player.connect_previous(move |_| {
                                let _ = tx.send(MprisAction::Previous);
                            });

                            let tx = action_tx.clone();
                            player.connect_seek(move |_, offset| {
                                let _ = tx.send(MprisAction::SeekRelative(offset.as_micros()));
                            });

                            let tx = action_tx.clone();
                            player.connect_set_position(move |_, _track_id, pos| {
                                let _ = tx.send(MprisAction::SetPosition(Duration::from_micros(
                                    pos.as_micros().max(0) as u64,
                                )));
                            });

                            let tx = action_tx.clone();
                            player.connect_set_volume(move |_, vol| {
                                let _ = tx.send(MprisAction::SetVolume(vol));
                            });

                            let tx = action_tx.clone();
                            player.connect_set_loop_status(move |_, status| {
                                let _ = tx.send(MprisAction::SetLoopStatus(status));
                            });

                            let tx = action_tx.clone();
                            player.connect_set_shuffle(move |_, shuffle| {
                                let _ = tx.send(MprisAction::SetShuffle(shuffle));
                            });

                            let server_task = tokio::task::spawn_local(player.run());

                            while let Some(update) = update_rx.recv().await {
                                match update {
                                    MprisUpdate::Track {
                                        id,
                                        title,
                                        artist,
                                        album,
                                        duration,
                                    } => {
                                        let path = format!("/org/mpui/track/{}", id.0);
                                        let track_id = TrackId::try_from(path.as_str())
                                            .unwrap_or(TrackId::NO_TRACK);
                                        let metadata = Metadata::builder()
                                            .trackid(track_id)
                                            .title(title)
                                            .artist(vec![artist])
                                            .album(album)
                                            .length(Time::from_micros(duration.as_micros() as i64))
                                            .build();
                                        let _ = player.set_metadata(metadata).await;
                                        let _ = player
                                            .set_playback_status(PlaybackStatus::Playing)
                                            .await;
                                        player.set_position(Time::ZERO);
                                    }
                                    MprisUpdate::Status(status) => {
                                        let _ = player.set_playback_status(status).await;
                                    }
                                    MprisUpdate::Position(pos) => {
                                        player.set_position(Time::from_micros(pos.as_micros() as i64));
                                    }
                                    MprisUpdate::Seeked(pos) => {
                                        let _ = player
                                            .seeked(Time::from_micros(pos.as_micros() as i64))
                                            .await;
                                    }
                                    MprisUpdate::Volume(vol) => {
                                        let _ = player.set_volume(vol).await;
                                    }
                                    MprisUpdate::Repeat(mode) => {
                                        let status = match mode {
                                            RepeatMode::Off => LoopStatus::None,
                                            RepeatMode::Track => LoopStatus::Track,
                                            RepeatMode::Queue => LoopStatus::Playlist,
                                        };
                                        let _ = player.set_loop_status(status).await;
                                    }
                                    MprisUpdate::Shuffle(shuf) => {
                                        let _ = player.set_shuffle(shuf).await;
                                    }
                                }
                            }

                            server_task.abort();
                        })
                        .await;
                });
            })
            .ok()?;

        Some(Self {
            action_rx,
            update_tx,
        })
    }
}
