use std::time::Duration;
use chan::Sender;

use config::Config;
use errors::*;
use scheduler::Task;
use input::I3BarEvent;
use block::{Block, ConfigBlock};
use widgets::text::TextWidget;
use widgets::button::ButtonWidget;
use widget::{I3BarWidget, State};

use blocks::dbus::Connection;
use uuid::Uuid;

use super::mbackend;
use super::utils;

pub struct StaticMusic {
    id: String,
    current_song: TextWidget,
    prev: Option<ButtonWidget>,
    play: Option<ButtonWidget>,
    next: Option<ButtonWidget>,
    dbus_conn: Connection,
    player_avail: bool,
    player: String,
    max_width: usize,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct StaticMusicConfig {
    /// Name of the music player.Must be the same name the player<br/> is registered with the MediaPlayer2 Interface.
    pub player: String,

    /// Max width of the block in characters, not including the buttons
    #[serde(default = "StaticMusicConfig::default_max_width")]
    pub max_width: usize,
    
    /// Array of control buttons to be displayed. Options are<br/>prev (previous title), play (play/pause) and next (next title)
    #[serde(default = "StaticMusicConfig::default_buttons")]
    pub buttons: Vec<String>,
}

impl StaticMusicConfig {
    fn default_max_width() -> usize {
        21
    }

    fn default_buttons() -> Vec<String> {
        vec![]
    }
}

impl ConfigBlock for StaticMusic {
    type Config = StaticMusicConfig;

    fn new(block_config: Self::Config, config: Config, send: Sender<Task>) -> Result<Self> {
        let id: String = Uuid::new_v4().simple().to_string();
        let listener_id = id.clone();
        mbackend::spawn_listener(listener_id, send);
        
        let (play, prev, next) = utils::create_buttons(&block_config.buttons, &config)?;
        
        Ok(StaticMusic {
            id,
            current_song: TextWidget::new(
                config.clone(),
            ).with_icon("music")
                .with_state(State::Info),
            prev,
            play,
            next,
            dbus_conn: mbackend::dbus_connection()?,
            player_avail: false,
            player: block_config.player,
            max_width: block_config.max_width,
        })
    }
}

impl Block for StaticMusic {
    fn id(&self) -> &str {
        &self.id
    }

    fn update(&mut self) -> Result<Option<Duration>> {
        let player_conn = mbackend::player_connection(&self.dbus_conn, &self.player);
        let data = mbackend::music_data(&player_conn);

        if data.is_err() {
            self.current_song.set_text(String::from(""));
            self.player_avail = false;
            self.current_song.set_icon("");
        } else {
            let metadata = data.unwrap();

            let (mut title, mut artist) = mbackend::extract_from_metadata(&metadata).unwrap_or((String::new(), String::new()));

            if title.is_empty() && artist.is_empty() {
                self.player_avail = false;
                self.current_song.set_text(String::new());
                self.current_song.set_icon("");
            } else {
                self.player_avail = true;
                self.current_song.set_icon("music");

                // From config
                let max = self.max_width;

                if title.is_empty() {
                    // Only display artist, truncated appropriately
                    self.current_song.set_text({
                        match artist.char_indices().nth(max) {
                            None => artist.to_string(),
                            Some((i, _)) => {artist.truncate(i);
                                             artist.to_string()}
                    }});

                    
                }
                else if artist.is_empty() {
                    // Only display title, truncated appropriately
                    self.current_song.set_text({
                        match title.char_indices().nth(max) {
                            None => title.to_string(),
                            Some((i, _)) => {title.truncate(i);
                                             title.to_string()}
                    }});
                }
                else {
                    let text = format!("{} - {}", title, artist);
                    let textlen = text.chars().count();
                    if textlen > max {
                        // overshoot: # of chars we need to trim
                        // substance: # of chars available for trimming
                        let overshoot = (textlen - max) as f32;
                        let substance = (textlen - 3) as f32;
                        
                        // Calculate number of chars to trim from title
                        let tlen = title.chars().count();
                        let tblm = tlen as f32 / substance;
                        let mut tnum = (overshoot * tblm).ceil() as usize;
                        
                        // Calculate number of chars to trim from artist
                        let alen = artist.chars().count();
                        let ablm = alen as f32 / substance;
                        let mut anum = (overshoot * ablm).ceil() as usize;
                        
                        // Prefer to only trim one of the title and artist

                        if anum < tnum && anum <= 3 && (tnum + anum < tlen) {
                            anum = 0;
                            tnum += anum;
                        }

                        if tnum < anum && tnum <= 3 && (anum + tnum < alen) {
                            tnum = 0;
                            anum += tnum;
                        }

                        // Calculate how many chars to keep from title and artist
                        
                        let mut ttrc = tlen - tnum;
                        if ttrc < 1 || ttrc > 5000 { ttrc = 1 }
                        
                        let mut atrc = alen - anum;
                        if atrc < 1 || atrc > 5000 { atrc = 1 }

                        // Truncate artist and title to appropriate lengths
                        
                        let tidx = title.char_indices().nth(ttrc).unwrap_or((title.len(), 'a')).0;
                        title.truncate(tidx);
                        
                        let aidx = artist.char_indices().nth(atrc).unwrap_or((artist.len(),'a')).0;
                        artist.truncate(aidx);

                        // Produce final formatted string

                        self.current_song.set_text(
                                 format!("{} | {}", title, artist));
                    }
                    else {
                        self.current_song.set_text(text);
                    }
                }
            }
        }
        if let Some(ref mut play) = self.play {
            let pb_data = mbackend::playback_data(&player_conn);
            utils::update_play_button(play, &pb_data);
        }
        Ok(Some(Duration::new(1, 0)))
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


