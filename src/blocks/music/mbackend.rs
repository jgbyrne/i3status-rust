use std::time::Instant;
use chan::Sender;
use std::thread;
use std::boxed::Box;
use std::result;

use errors::*;
use scheduler::Task;

use blocks::dbus::{Error, arg, stdintf, BusType, Connection, ConnPath, ConnectionItem, Message};
use self::stdintf::OrgFreedesktopDBusProperties;

/// Spawn a thread to alert on changes to the player state
pub fn spawn_listener(id: String, send: Sender<Task>) {
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
}

/// Establish a connection to the D-Bus
pub fn dbus_connection() -> Result<Connection> {
    Connection::get_private(BusType::Session)
                .block_error("music", "failed to establish D-Bus connection")
}

/// Obtain a connection to the media player interface
pub fn player_connection<'c>(dbus_conn: &'c Connection, player: &str) -> ConnPath<'c, &'c Connection> {
    dbus_conn.with_path(
        format!("org.mpris.MediaPlayer2.{}", player),
        "/org/mpris/MediaPlayer2",
        1000,
    )
}

/// Type alias for data returned by ConnPath.get(....)
pub type PlayerData = arg::Variant<Box<arg::RefArg>>;

/// Get information about currently playing music
pub fn music_data<'c>(player_conn: &ConnPath<&'c Connection>) -> result::Result<PlayerData, Error> {
    player_conn.get("org.mpris.MediaPlayer2.Player", "Metadata")
}

/// Get information about current playback state
pub fn playback_data<'c>(player_conn: &ConnPath<&'c Connection>) -> result::Result<PlayerData, Error> {
    player_conn.get("org.mpris.MediaPlayer2.Player", "PlaybackStatus")
}


/// Pull artist, title pair from music data
pub fn extract_from_metadata(metadata: &PlayerData) -> Result<(String, String)> {
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

fn music_action(player: &str, dbus_conn: &mut Connection, action: &str) -> Result<()> {
    if action != "" {
        let m = Message::new_method_call(
            format!("org.mpris.MediaPlayer2.{}", player),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
            action,
        ).block_error("music", "failed to create D-Bus method call")?;
        dbus_conn
            .send(m)
            .block_error("music", "failed to call method via D-Bus")
            .map(|_| ())
    } else {
        Ok(())
    }
}

pub fn music_play(player: &str, dbus_conn: &mut Connection) -> Result<()>{
    music_action(player, dbus_conn, "PlayPause")
}

pub fn music_next(player: &str, dbus_conn: &mut Connection) -> Result<()>{
    music_action(player, dbus_conn, "Next")
}

pub fn music_prev(player: &str, dbus_conn: &mut Connection) -> Result<()>{
    music_action(player, dbus_conn, "Previous")
}


