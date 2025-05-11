use std::thread;
use std::time::Duration;
use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};

use crate::models::UncivSound;
use crate::models::metadata::GameSettings;
use crate::ui::audio::{MusicController, MusicTrackChooserFlags};
use crate::ui::components::widgets::{Button, ImageButton, Label, Slider, WrappableLabel};
use crate::ui::popups::options::OptionsPopup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::utils::concurrency::Concurrency;
use crate::utils::translation::tr;
use crate::UncivGame;

/// Creates the sound tab for the options popup
pub fn sound_tab(options_popup: &OptionsPopup) -> BaseScreen {
    let mut table = BaseScreen::new();
    table.pad(10.0);
    table.defaults().pad(5.0);

    let settings = &options_popup.settings;
    let music = UncivGame::current().music_controller();

    add_sound_effects_volume_slider(&mut table, settings);
    add_city_sounds_volume_slider(&mut table, settings);

    if UncivGame::current().music_controller().is_voices_available() {
        add_voices_volume_slider(&mut table, settings);
    }

    if UncivGame::current().music_controller().is_music_available() {
        add_music_controls(&mut table, settings, music);
    }

    if !UncivGame::current().music_controller().is_default_file_available() {
        add_download_music(&mut table, options_popup);
    }

    table
}

/// Adds a download music button to the table
fn add_download_music(table: &mut BaseScreen, options_popup: &OptionsPopup) {
    let mut download_music_button = Button::new("Download music");
    table.add(&download_music_button).colspan(2).row();

    let mut error_table = BaseScreen::new();
    table.add(&error_table).colspan(2).row();

    download_music_button.set_on_click(Box::new(move || {
        download_music_button.set_enabled(false);
        error_table.clear();
        error_table.add(Label::new("Downloading..."));

        // So the whole game doesn't get stuck while downloading the file
        Concurrency::run("MusicDownload", move || {
            match UncivGame::current().music_controller().download_default_file() {
                Ok(_) => {
                    // Launch on GL thread
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(100));
                        // This would be replaced with a proper GL thread launch mechanism
                        // in the actual implementation
                        options_popup.tabs.replace_page("Sound", sound_tab(options_popup));
                        UncivGame::current().music_controller().choose_track(MusicTrackChooserFlags::set_play_default);
                    });
                },
                Err(_) => {
                    // Launch on GL thread
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(100));
                        // This would be replaced with a proper GL thread launch mechanism
                        error_table.clear();
                        error_table.add(Label::new_with_color("Could not download music!", Color::RED));
                    });
                }
            }
        });
    }));
}

/// Adds a volume slider to a table
fn add_volume_slider(
    table: &mut BaseScreen,
    text: &str,
    initial: f32,
    silent: bool,
    on_change: Box<dyn FnMut(f32)>
) {
    table.add(Label::new(tr(text))).left().fill_x();

    let mut volume_slider = Slider::new(
        0.0, 1.0, 0.05,
        initial,
        if silent { UncivSound::Silent } else { UncivSound::Slider },
        Box::new(Slider::format_percent),
        on_change
    );

    table.add(&volume_slider).pad(5.0).row();
}

/// Adds a sound effects volume slider to a table
fn add_sound_effects_volume_slider(table: &mut BaseScreen, settings: &GameSettings) {
    add_volume_slider(
        table,
        "Sound effects volume",
        settings.sound_effects_volume,
        false,
        Box::new(move |value| {
            settings.sound_effects_volume = value;
        })
    );
}

/// Adds a city sounds volume slider to a table
fn add_city_sounds_volume_slider(table: &mut BaseScreen, settings: &GameSettings) {
    add_volume_slider(
        table,
        "City ambient sound volume",
        settings.city_sounds_volume,
        false,
        Box::new(move |value| {
            settings.city_sounds_volume = value;
        })
    );
}

/// Adds a voices volume slider to a table
fn add_voices_volume_slider(table: &mut BaseScreen, settings: &GameSettings) {
    add_volume_slider(
        table,
        "Leader voices volume",
        settings.voices_volume,
        false,
        Box::new(move |value| {
            settings.voices_volume = value;
        })
    );
}

/// Adds a music volume slider to a table
fn add_music_volume_slider(table: &mut BaseScreen, settings: &GameSettings, music: &MusicController) {
    add_volume_slider(
        table,
        "Music volume",
        settings.music_volume,
        true,
        Box::new(move |value| {
            settings.music_volume = value;
            music.set_volume(value);

            if !music.is_playing() {
                music.choose_track(MusicTrackChooserFlags::set_play_default);
            }
        })
    );
}

/// Adds a music pause slider to a table
fn add_music_pause_slider(table: &mut BaseScreen, settings: &GameSettings, music: &MusicController) {
    // map to/from 0-1-2..10-12-14..30-35-40..60-75-90-105-120
    fn pos_to_length(pos: f32) -> f32 {
        match pos {
            pos if pos >= 0.0 && pos <= 10.0 => pos,
            pos if pos >= 11.0 && pos <= 20.0 => pos * 2.0 - 10.0,
            pos if pos >= 21.0 && pos <= 26.0 => pos * 5.0 - 70.0,
            _ => pos * 15.0 - 330.0
        }
    }

    fn length_to_pos(length: f32) -> f32 {
        let pos = match length {
            length if length >= 0.0 && length <= 10.0 => length,
            length if length >= 11.0 && length <= 30.0 => (length + 10.0) / 2.0,
            length if length >= 31.0 && length <= 60.0 => (length + 10.0) / 5.0,
            _ => (length + 330.0) / 15.0
        };
        pos.floor()
    }

    let get_tip_text = |value: f32| -> String {
        format!("{:.0}", pos_to_length(value))
    };

    table.add(Label::new(tr("Pause between tracks"))).left().fill_x();

    let mut pause_length_slider = Slider::new(
        0.0, 30.0, 1.0,
        length_to_pos(music.silence_length),
        UncivSound::Silent,
        Box::new(get_tip_text),
        Box::new(move |value| {
            music.silence_length = pos_to_length(value);
            settings.pause_between_tracks = music.silence_length as i32;
        })
    );

    table.add(&pause_length_slider).pad(5.0).row();
}

/// Adds a currently playing label to a table
fn add_music_currently_playing(table: &mut BaseScreen, music: &MusicController) {
    let mut label = WrappableLabel::new("", table.width() - 10.0, Color::new(0.0, 0.5, 0.0, 1.0), 16);
    label.set_wrap(true);
    table.add(&label).pad_top(20.0).colspan(2).fill_x().row();

    music.set_on_change(Box::new(move |track_name| {
        label.set_text(format!("Currently playing: [{}]", tr(track_name)));
    }));

    // This would be handled differently in Rust, likely with a drop guard or similar pattern
    // For now, we'll just leave it as a comment
    // table.first_ascendant(Popup::class.java)?.run {
    //     close_listeners.add { music.onChange(null) }
    // }
}

/// Adds simple player controls to a table
fn add_simple_player_controls(table: &mut BaseScreen, music: &MusicController) {
    fn create_image_button(path: &str, over_color: Color) -> ImageButton {
        ImageButton::new(path, 30.0, 30.0, Color::CLEAR, over_color)
    }

    let mut controls_table = BaseScreen::new();
    controls_table.defaults().space(25.0);

    let mut pause_button = create_image_button("OtherIcons/Pause", Color::GOLD);
    pause_button.set_on_click(Box::new(move || {
        music.pause(0.5);
    }));
    controls_table.add(&pause_button);

    let mut forward_button = create_image_button("OtherIcons/ForwardArrow", Color::LIME);
    forward_button.set_on_click(Box::new(move || {
        music.resume(0.5);
    }));
    controls_table.add(&forward_button);

    let mut loading_button = create_image_button("OtherIcons/Loading", Color::VIOLET);
    loading_button.set_on_click(Box::new(move || {
        music.choose_track(MusicTrackChooserFlags::none);
    }));
    controls_table.add(&loading_button);

    table.add(&controls_table).colspan(2).center().row();
}

/// Adds music volume/pause sliders, currently playing label and player controls to a table
pub fn add_music_controls(table: &mut BaseScreen, settings: &GameSettings, music: &MusicController) {
    add_music_volume_slider(table, settings, music);
    add_music_pause_slider(table, settings, music);
    add_music_currently_playing(table, music);
    add_simple_player_controls(table, music);
}