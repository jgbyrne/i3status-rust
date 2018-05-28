use std::result;
use config::Config;
use errors::*;
use widgets::button::ButtonWidget;
use widget::{I3BarWidget, State};
use blocks::dbus::Error;
use blocks::music::mbackend::PlayerData;

pub fn create_buttons(buttons: &[String], config: &Config)
        -> Result<(Option<ButtonWidget>, Option<ButtonWidget>, Option<ButtonWidget>)> {
    
    let mut play: Option<ButtonWidget> = None;
    let mut prev: Option<ButtonWidget> = None;
    let mut next: Option<ButtonWidget> = None;
            
    for button in buttons {
        match button.as_ref() {
            "play" => {
                play = Some(
                    ButtonWidget::new(config.clone(), "play")
                        .with_icon("music_play")
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
            "next" => {
                next = Some(
                    ButtonWidget::new(config.clone(), "next")
                        .with_icon("music_next")
                        .with_state(State::Info),
                )
            }
            x => Err(BlockError(
                "music".to_owned(),
                format!("unknown music button identifier: '{}'", x),
            ))?,
        };
    }
    Ok((play, prev, next))
}

pub fn generate_view<'w>(player_avail: bool,
                     current_song: &'w I3BarWidget,
                     play: &'w Option<ButtonWidget>,
                     prev: &'w Option<ButtonWidget>,
                     next: &'w Option<ButtonWidget>)
                   -> Vec<&'w I3BarWidget> {
    if player_avail {
        let mut elements: Vec<&I3BarWidget> = Vec::new();
        elements.push(current_song);
        if let Some(ref prev) = prev {
            elements.push(prev);
        }
        if let Some(ref play) = play {
            elements.push(play);
        }
        if let Some(ref next) = next {
            elements.push(next);
        }
        elements
    } else {
        vec![current_song]
    }
}

pub fn update_play_button(play: &mut ButtonWidget, data: &result::Result<PlayerData, Error>) {
    match data {
        Err(_) => play.set_icon("music_play"),
        Ok(data) => {
            let state = &data.0;
            if state.as_str().map(|s| s != "Playing").unwrap_or(false) {
                play.set_icon("music_play")
            } else {
                play.set_icon("music_pause")
            }
        }
    }
}
