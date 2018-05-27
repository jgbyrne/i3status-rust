use std::time::{Duration, Instant};
use chan::Sender;
use std::thread;
use std::boxed::Box;

use config::Config;
use errors::*;
use scheduler::Task;
use input::I3BarEvent;
use block::{Block, ConfigBlock};
use widgets::text::TextWidget;
use widgets::button::ButtonWidget;
use widget::{I3BarWidget, State};

use blocks::dbus::{arg, stdintf, BusType, Connection, ConnectionItem, Message};
use self::stdintf::OrgFreedesktopDBusProperties;
use uuid::Uuid;

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
        let id_copy = id.clone();

        thread::spawn(move || {
            let c = Connection::get_private(BusType::Session).unwrap();
            c.add_match(
                "interface='org.freedesktop.DBus.Properties',member='PropertiesChanged'",
            ).unwrap();
            loop {
                for ci in c.iter(100000) {
                    if let ConnectionItem::Signal(msg) = ci {
                        if &*msg.path().unwrap() == "/org/mpris/MediaPlayer2" && &*msg.member().unwrap() == "PropertiesChanged" {
                            send.send(Task {
                                id: id.clone(),
                                update_time: Instant::now(),
                            });
                        }
                    }
                }
            }
        });

        let mut play: Option<ButtonWidget> = None;
        let mut prev: Option<ButtonWidget> = None;
        let mut next: Option<ButtonWidget> = None;
        for button in block_config.buttons {
            match &*button {
                "play" => {
                    play = Some(
                        ButtonWidget::new(config.clone(), "play")
                            .with_icon("music_play")
                            .with_state(State::Info),
                    )
                }
                "next" => {
                    next = Some(
                        ButtonWidget::new(config.clone(), "next")
                            .with_icon("music_next")
                            .with_state(State::Info),
                    )
                }
                "prev" => {
                    prev = Some(
                        ButtonWidget::new(config.clone(), "prev")
                            .with_icon("music_prev")
                            .with_state(State::Info),
                    )
                }
                x => Err(BlockError(
                    "music".to_owned(),
                    format!("unknown music button identifier: '{}'", x),
                ))?,
            };
        }

        Ok(StaticMusic {
            id: id_copy,
            current_song: TextWidget::new(
                config.clone(),
            ).with_icon("music")
                .with_state(State::Info),
            prev: prev,
            play: play,
            next: next,
            dbus_conn: Connection::get_private(BusType::Session)
                .block_error("music", "failed to establish D-Bus connection")?,
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
        let c = self.dbus_conn.with_path(
            format!("org.mpris.MediaPlayer2.{}", self.player),
            "/org/mpris/MediaPlayer2",
            1000,
        );
        let data = c.get("org.mpris.MediaPlayer2.Player", "Metadata");

        if data.is_err() {
            self.current_song.set_text(String::from(""));
            self.player_avail = false;
        } else {
            let metadata = data.unwrap();

            let (mut title, mut artist) = extract_from_metadata(&metadata).unwrap_or((String::new(), String::new()));

            if title.is_empty() && artist.is_empty() {
                self.player_avail = false;
                self.current_song.set_text(String::new());
            } else {
                self.player_avail = true;

                // From config
                let max = self.max_width;

                if title.is_empty() {
                    // Only display artist, truncated appropriately
                    self.current_song.set_text({
                        match artist.char_indices().nth(max) {
                            None => format!("{}", artist),
                            Some((i, _)) => {artist.truncate(i);
                                             format!("{}", artist)}
                    }});
                }
                else if artist.is_empty() {
                    // Only display title, truncated appropriately
                    self.current_song.set_text({
                        match title.char_indices().nth(max) {
                            None => format!("{}", title),
                            Some((i, _)) => {title.truncate(i);
                                             format!("{}", title)}
                    }});
                }
                else {
                    let text = format!("{} | {}", title, artist);
                    if text.chars().count() > max {
                        
                        // overshoot: # of chars we need to trim
                        // substance: # of chars available for trimming
                        let overshoot = (text.chars().count() - max) as f32;
                        let substance = (text.chars().count() - 3) as f32;
                        
                        // Calculate number of chars to trim from title
                        let tlen = title.chars().count();
                        let tblm = tlen as f32 / substance;
                        let mut tnum = (overshoot * tblm).ceil() as usize;
                        
                        // Calculate number of chars to trim from artist
                        let alen = artist.chars().count();
                        let ablm = alen as f32 / substance;
                        let mut anum = (overshoot * ablm).ceil() as usize;
                        
                        // Prefer to only trim one of the title and artist

                        if (anum < tnum && anum <= 3 && (tnum + anum < tlen)) {
                            anum = 0;
                            tnum += anum;
                        }

                        if (tnum < anum && tnum <= 3 && (anum + tnum < alen)) {
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
            let data = c.get("org.mpris.MediaPlayer2.Player", "PlaybackStatus");
            match data {
                Err(_) => play.set_icon("music_play"),
                Ok(data) => {
                    let state = data.0;
                    if state.as_str().map(|s| s != "Playing").unwrap_or(false) {
                        play.set_icon("music_play")
                    } else {
                        play.set_icon("music_pause")
                    }
                }
            }
        }
        Ok(Some(Duration::new(1, 0)))
    }

    fn click(&mut self, event: &I3BarEvent) -> Result<()> {
        if let Some(ref name) = event.name {
            let action = match name as &str {
                "play" => "PlayPause",
                "next" => "Next",
                "prev" => "Previous",
                _ => "",
            };
            if action != "" {
                let m = Message::new_method_call(
                    format!("org.mpris.MediaPlayer2.{}", self.player),
                    "/org/mpris/MediaPlayer2",
                    "org.mpris.MediaPlayer2.Player",
                    action,
                ).block_error("music", "failed to create D-Bus method call")?;
                self.dbus_conn
                    .send(m)
                    .block_error("music", "failed to call method via D-Bus")
                    .map(|_| ())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn view(&self) -> Vec<&I3BarWidget> {
        if self.player_avail {
            let mut elements: Vec<&I3BarWidget> = Vec::new();
            elements.push(&self.current_song);
            if let Some(ref prev) = self.prev {
                elements.push(prev);
            }
            if let Some(ref play) = self.play {
                elements.push(play);
            }
            if let Some(ref next) = self.next {
                elements.push(next);
            }
            elements
        } else {
            vec![&self.current_song]
        }
    }
}

fn extract_from_metadata(metadata: &arg::Variant<Box<arg::RefArg>>) -> Result<(String, String)> {
    let mut title = String::new();
    let mut artist = String::new();

    let mut iter = metadata
        .0
        .as_iter()
        .block_error("music", "failed to extract metadata")?;

    while let Some(key) = iter.next() {
        let value = iter.next()
            .block_error("music", "failed to extract metadata")?;
        match key.as_str()
            .block_error("music", "failed to extract metadata")?
        {
            "xesam:artist" => {
                artist = String::from(value
                    .as_iter()
                    .block_error("music", "failed to extract metadata")?
                    .nth(0)
                    .block_error("music", "failed to extract metadata")?
                    .as_iter()
                    .block_error("music", "failed to extract metadata")?
                    .nth(0)
                    .block_error("music", "failed to extract metadata")?
                    .as_iter()
                    .block_error("music", "failed to extract metadata")?
                    .nth(0)
                    .block_error("music", "failed to extract metadata")?
                    .as_str()
                    .block_error("music", "failed to extract metadata")?)
            }
            "xesam:title" => {
                title = String::from(value
                    .as_str()
                    .block_error("music", "failed to extract metadata")?)
            }
            _ => {}
        };
    }
    Ok((title, artist))
}
