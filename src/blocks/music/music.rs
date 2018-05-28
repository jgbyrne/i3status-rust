use std::time::Duration;
use chan::Sender;

use config::Config;
use errors::*;
use scheduler::Task;
use input::I3BarEvent;
use block::{Block, ConfigBlock};
use de::deserialize_duration;
use widgets::rotatingtext::RotatingTextWidget;
use widgets::button::ButtonWidget;
use widget::{I3BarWidget, State};

use blocks::dbus::Connection;
use uuid::Uuid;

use super::mbackend;
use super::utils;

pub struct Music {
    id: String,
    current_song: RotatingTextWidget,
    prev: Option<ButtonWidget>,
    play: Option<ButtonWidget>,
    next: Option<ButtonWidget>,
    dbus_conn: Connection,
    player_avail: bool,
    marquee: bool,
    player: String,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct MusicConfig {
    /// Name of the music player.Must be the same name the player<br/> is registered with the MediaPlayer2 Interface.
    pub player: String,

    /// Max width of the block in characters, not including the buttons
    #[serde(default = "MusicConfig::default_max_width")]
    pub max_width: usize,

    /// Bool to specify if a marquee style rotation should be used<br/> if the title + artist is longer than max-width
    #[serde(default = "MusicConfig::default_marquee")]
    pub marquee: bool,

    /// Marquee interval in seconds. This is the delay between each rotation.
    #[serde(default = "MusicConfig::default_marquee_interval", deserialize_with = "deserialize_duration")]
    pub marquee_interval: Duration,

    /// Marquee speed in seconds. This is the scrolling time used per character.
    #[serde(default = "MusicConfig::default_marquee_speed", deserialize_with = "deserialize_duration")]
    pub marquee_speed: Duration,

    /// Array of control buttons to be displayed. Options are<br/>prev (previous title), play (play/pause) and next (next title)
    #[serde(default = "MusicConfig::default_buttons")]
    pub buttons: Vec<String>,
}

impl MusicConfig {
    fn default_max_width() -> usize {
        21
    }

    fn default_marquee() -> bool {
        true
    }

    fn default_marquee_interval() -> Duration {
        Duration::from_secs(10)
    }

    fn default_marquee_speed() -> Duration {
        Duration::from_millis(500)
    }

    fn default_buttons() -> Vec<String> {
        vec![]
    }
}

impl ConfigBlock for Music {
    type Config = MusicConfig;

    fn new(block_config: Self::Config, config: Config, send: Sender<Task>) -> Result<Self> {
        let id: String = Uuid::new_v4().simple().to_string();
        let listener_id = id.clone();
        mbackend::spawn_listener(listener_id, send);
        
        let (play, prev, next) = utils::create_buttons(&block_config.buttons, &config)?;
        
        Ok(Music {
            id: id,
            current_song: RotatingTextWidget::new(
                Duration::new(block_config.marquee_interval.as_secs(), 0),
                Duration::new(0, block_config.marquee_speed.subsec_nanos()),
                block_config.max_width,
                config.clone(),
            ).with_icon("music")
                .with_state(State::Info),
            prev: prev,
            play: play,
            next: next,
            dbus_conn: mbackend::dbus_connection()?,
            player_avail: false,
            player: block_config.player,
            marquee: block_config.marquee,
        })
    }
}

impl Block for Music {
    fn id(&self) -> &str {
        &self.id
    }

    fn update(&mut self) -> Result<Option<Duration>> {
        let (rotated, next) = if self.marquee {
            self.current_song.next()?
        } else {
            (false, None)
        };

        if !rotated {
            let player_conn = mbackend::player_connection(&self.dbus_conn, &self.player);
            let data = mbackend::music_data(&player_conn);

            if data.is_err() {
                self.current_song.set_text(String::from(""));
                self.player_avail = false;
            } else {
                let metadata = data.unwrap();

                let (title, artist) = mbackend::extract_from_metadata(&metadata).unwrap_or((String::new(), String::new()));

                if title.is_empty() && artist.is_empty() {
                    self.player_avail = false;
                    self.current_song.set_text(String::new());
                } else {
                    self.player_avail = true;
                    self.current_song
                        .set_text(format!("{} | {}", title, artist));
                }
            }
            if let Some(ref mut play) = self.play {
                let pb_data = mbackend::playback_data(&player_conn);
                utils::update_play_button(play, &pb_data);
            }
        }
        Ok(match (next, self.marquee) {
            (Some(_), _) => next,
            (None, true) => Some(Duration::new(1, 0)),
            (None, false) => Some(Duration::new(1, 0)),
        })
    }

    fn click(&mut self, event: &I3BarEvent) -> Result<()> {
        if let Some(ref name) = event.name {
            match name as &str {
                "play" => mbackend::music_play(&self.player, &mut self.dbus_conn),
                "next" => mbackend::music_next(&self.player, &mut self.dbus_conn),
                "prev" => mbackend::music_prev(&self.player, &mut self.dbus_conn),
                _ => Ok(()),
            }?
            
        }
        Ok(())
    }

    fn view(&self) -> Vec<&I3BarWidget> {
        utils::generate_view(self.player_avail,
                             &self.current_song,
                             &self.play,
                             &self.prev,
                             &self.next)
    }
}
