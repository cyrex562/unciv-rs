use std::collections::HashMap;
use std::time::Duration;
use std::sync::Arc;

use ggez::graphics::{Color, DrawParam, Text};
use ggez::input::keyboard::KeyCode;
use ggez::mint::Point2;
use ggez::timer::Timer;

use crate::ui::components::input::{KeyCharAndCode, KeyShortcutDispatcher};
use crate::ui::components::widgets::slider::Slider;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::components::widgets::widget_group::WidgetGroup;
use crate::ui::components::widgets::table::Table;
use crate::ui::components::widgets::label::Label;
use crate::ui::components::widgets::icon_circle_group::IconCircleGroup;
use crate::ui::components::widgets::container::Container;
use crate::ui::components::widgets::scroll_pane::ScrollPane;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::translations::TranslationManager;
use crate::ui::audio::SoundPlayer;
use crate::ui::models::UncivSound;
use crate::ui::constants::Constants;

/// A modified slider with additional features
pub struct UncivSlider {
    /// The base Table that this UncivSlider extends
    base: Table,

    /// The inner slider
    slider: Slider,

    /// The minus button
    minus_button: Option<IconCircleGroup>,

    /// The plus button
    plus_button: Option<IconCircleGroup>,

    /// The tip label
    tip_label: Label,

    /// The tip container
    tip_container: Container<Label>,

    /// The tip hide task
    tip_hide_task: Option<Timer>,

    /// The snap to values
    snap_to_values: Option<Vec<f32>>,

    /// The snap threshold
    snap_threshold: f32,

    /// The tip format
    tip_format: String,

    /// Whether the slider has focus
    has_focus: bool,

    /// Whether to block listener events
    block_listener: bool,

    /// The tip type
    tip_type: TipType,

    /// The get tip text function
    get_tip_text: Option<Arc<dyn Fn(f32) -> String + Send + Sync>>,

    /// The on change function
    on_change: Option<Arc<dyn Fn(f32) + Send + Sync>>,

    /// The killed listeners
    killed_listeners: HashMap<ScrollPane, Vec<Box<dyn Fn() + Send + Sync>>>,

    /// The killed capture listeners
    killed_capture_listeners: HashMap<ScrollPane, Vec<Box<dyn Fn() + Send + Sync>>>,
}

/// The tip type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipType {
    /// No tip
    None,
    /// Auto tip
    Auto,
    /// Permanent tip
    Permanent,
}

impl UncivSlider {
    /// Constants for geometry tuning
    const PLUS_MINUS_FONT_SIZE: f32 = Constants::DEFAULT_FONT_SIZE as f32;
    const PLUS_MINUS_CIRCLE_SIZE: f32 = 20.0;
    const PADDING: f32 = 5.0; // padding around the Slider, doubled between it and +/- buttons
    const HIDE_DELAY: f32 = 3.0; // delay in s to hide tooltip
    const TIP_ANIMATION_DURATION: f32 = 0.2; // tip show/hide duration in s

    /// Creates a new UncivSlider with the given parameters
    pub fn new(
        min: f32,
        max: f32,
        step: f32,
        vertical: bool,
        plus_minus: bool,
        initial: f32,
        sound: UncivSound,
        tip_type: TipType,
        get_tip_text: Option<Arc<dyn Fn(f32) -> String + Send + Sync>>,
        on_change: Option<Arc<dyn Fn(f32) + Send + Sync>>,
    ) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        let slider = Slider::new(min, max, step, vertical, skin);
        let tip_label = Label::new("", Color::LIGHT_GRAY);
        let tip_container = Container::new(tip_label.clone());

        let mut unciv_slider = Self {
            base: Table::new(skin),
            slider,
            minus_button: None,
            plus_button: None,
            tip_label,
            tip_container,
            tip_hide_task: None,
            snap_to_values: None,
            snap_threshold: 0.0,
            tip_format: "%.1f".to_string(),
            has_focus: false,
            block_listener: false,
            tip_type,
            get_tip_text,
            on_change,
            killed_listeners: HashMap::new(),
            killed_capture_listeners: HashMap::new(),
        };

        // Initialize tip formatting
        unciv_slider.step_changed();

        // Add minus button if needed
        if plus_minus {
            let minus_button = Label::new("-", Color::WHITE, Self::PLUS_MINUS_FONT_SIZE)
                .with_alignment(ggez::graphics::Align::Center)
                .surround_with_circle(Self::PLUS_MINUS_CIRCLE_SIZE, true, skin.get_color("color"));

            minus_button.on_click(Box::new(move || {
                unciv_slider.add_to_value(-unciv_slider.step_size());
            }));

            unciv_slider.base.add(minus_button.clone()).with_padding(
                if vertical { (0.0, Self::PADDING, 0.0, 0.0) } else { (0.0, 0.0, 0.0, Self::PADDING) }
            );

            if vertical {
                unciv_slider.base.row();
            }

            unciv_slider.minus_button = Some(minus_button);
        }

        // Add slider
        unciv_slider.base.add(unciv_slider.slider.clone()).with_padding(Self::PADDING).fill_y().grow_x();

        // Add plus button if needed
        if plus_minus {
            if vertical {
                unciv_slider.base.row();
            }

            let plus_button = Label::new("+", Color::WHITE, Self::PLUS_MINUS_FONT_SIZE)
                .with_alignment(ggez::graphics::Align::Center)
                .surround_with_circle(Self::PLUS_MINUS_CIRCLE_SIZE, true, skin.get_color("color"));

            plus_button.on_click(Box::new(move || {
                unciv_slider.add_to_value(unciv_slider.step_size());
            }));

            unciv_slider.base.add(plus_button.clone()).with_padding(
                if vertical { (Self::PADDING, 0.0, 0.0, 0.0) } else { (0.0, 0.0, Self::PADDING, 0.0) }
            );

            unciv_slider.plus_button = Some(plus_button);
        }

        // Add listener to slider
        unciv_slider.slider.add_change_listener(Box::new(move |event, actor| {
            if unciv_slider.block_listener {
                return;
            }

            if unciv_slider.slider.is_dragging() != unciv_slider.has_focus {
                unciv_slider.has_focus = unciv_slider.slider.is_dragging();
                if unciv_slider.has_focus {
                    unciv_slider.kill_scroll_panes();
                } else {
                    unciv_slider.resurrect_scroll_panes();
                }
            }

            unciv_slider.value_changed();

            if let Some(on_change) = &unciv_slider.on_change {
                on_change(unciv_slider.slider.value());
            }

            SoundPlayer::play(sound);
        }));

        // Set initial value
        unciv_slider.set_value(initial);

        unciv_slider
    }

    /// Formats a value as a percentage
    pub fn format_percent(value: f32) -> String {
        format!("{}%", ((value * 100.0 + 0.5) as i32).to_string().tr())
    }

    /// Gets the minimum value
    pub fn min_value(&self) -> f32 {
        self.slider.min_value()
    }

    /// Gets the maximum value
    pub fn max_value(&self) -> f32 {
        self.slider.max_value()
    }

    /// Gets the current value
    pub fn value(&self) -> f32 {
        self.slider.value()
    }

    /// Sets the current value
    pub fn set_value(&mut self, new_value: f32) {
        self.block_listener = true;
        self.slider.set_value(new_value);
        self.block_listener = false;
        self.value_changed();
    }

    /// Gets the step size
    pub fn step_size(&self) -> f32 {
        self.slider.step_size()
    }

    /// Sets the step size
    pub fn set_step_size(&mut self, value: f32) {
        self.slider.set_step_size(value);
        self.step_changed();
    }

    /// Returns true if the slider is being dragged
    pub fn is_dragging(&self) -> bool {
        self.slider.is_dragging()
    }

    /// Gets whether the slider is disabled
    pub fn is_disabled(&self) -> bool {
        self.slider.is_disabled()
    }

    /// Sets whether the slider is disabled
    pub fn set_disabled(&mut self, value: bool) {
        self.slider.set_disabled(value);
        self.set_plus_minus_enabled();
    }

    /// Sets the range of this slider
    pub fn set_range(&mut self, min: f32, max: f32) {
        self.slider.set_range(min, max);
        self.set_plus_minus_enabled();
    }

    /// Sets the snap to values
    pub fn set_snap_to_values(&mut self, threshold: f32, values: &[f32]) {
        self.snap_to_values = Some(values.to_vec());
        self.snap_threshold = threshold;
        self.slider.set_snap_to_values(threshold, values);
    }

    /// Adds to the current value
    fn add_to_value(&mut self, delta: f32) {
        // Un-snapping with Shift is taken from Slider source
        if self.snap_to_values.is_none() || KeyCharAndCode::SHIFT.is_in_shortcuts() {
            self.set_value(self.value() + delta);
            if let Some(on_change) = &self.on_change {
                on_change(self.value());
            }
            return;
        }

        let snap_values = self.snap_to_values.as_ref().unwrap();
        let mut best_diff = -1.0;
        let mut best_index = -1;

        for (i, snap_value) in snap_values.iter().enumerate() {
            let diff = (self.value() - snap_value).abs();
            if diff <= self.snap_threshold {
                if best_index == -1 || diff < best_diff {
                    best_diff = diff;
                    best_index = i as i32;
                }
            }
        }

        best_index += delta.signum() as i32;
        if best_index < 0 || best_index >= snap_values.len() as i32 {
            return;
        }

        self.set_value(snap_values[best_index as usize]);
        if let Some(on_change) = &self.on_change {
            on_change(self.value());
        }
    }

    /// Called when the value changes
    fn value_changed(&mut self) {
        match self.tip_type {
            TipType::None => {},
            _ => {
                if let Some(get_tip_text) = &self.get_tip_text {
                    self.tip_label.set_text(get_tip_text(self.value()));
                } else {
                    self.tip_label.set_text(format!(self.tip_format, self.value()));
                }

                match self.tip_type {
                    TipType::None => {},
                    TipType::Auto => {
                        if !self.tip_hide_task.is_some() {
                            self.show_tip();
                        }

                        if let Some(timer) = &mut self.tip_hide_task {
                            timer.cancel();
                        }

                        let mut timer = Timer::new();
                        timer.schedule(Duration::from_secs_f32(Self::HIDE_DELAY), Box::new(move || {
                            self.hide_tip();
                        }));
                        self.tip_hide_task = Some(timer);
                    },
                    TipType::Permanent => {
                        self.show_tip();
                    },
                }
            },
        }

        self.set_plus_minus_enabled();
    }

    /// Sets the plus/minus buttons enabled state
    fn set_plus_minus_enabled(&mut self) {
        let enable_minus = self.value() > self.min_value() && !self.is_disabled();
        if let Some(minus_button) = &mut self.minus_button {
            minus_button.set_touchable(enable_minus);
            minus_button.circle_mut().color.a = if enable_minus { 1.0 } else { 0.5 };
        }

        let enable_plus = self.value() < self.max_value() && !this.is_disabled();
        if let Some(plus_button) = &mut this.plus_button {
            plus_button.set_touchable(enable_plus);
            plus_button.circle_mut().color.a = if enable_plus { 1.0 } else { 0.5 };
        }
    }

    /// Called when the step size changes
    fn step_changed(&mut self) {
        self.tip_format = if this.step_size() > 0.99 {
            "%.0f".to_string()
        } else if this.step_size() > 0.099 {
            "%.1f".to_string()
        } else if this.step_size() > 0.0099 {
            "%.2f".to_string()
        } else {
            "%.3f".to_string()
        };

        if this.tip_type != TipType::None && this.get_tip_text.is_none() {
            this.tip_label.set_text(format!(this.tip_format, this.value()));
        }
    }

    /// Kills the scroll panes
    fn kill_scroll_panes(&mut self) {
        let mut widget: &dyn WidgetGroup = this;
        while let Some(parent) = widget.parent() {
            widget = parent;
            if !widget.is_scroll_pane() {
                continue;
            }

            let scroll_pane = widget.as_scroll_pane().unwrap();
            if !scroll_pane.listeners().is_empty() {
                this.killed_listeners.insert(scroll_pane.clone(), scroll_pane.listeners().to_vec());
            }

            if !scroll_pane.capture_listeners().is_empty() {
                this.killed_capture_listeners.insert(scroll_pane.clone(), scroll_pane.capture_listeners().to_vec());
            }

            scroll_pane.clear_listeners();
        }
    }

    /// Resurrects the scroll panes
    fn resurrect_scroll_panes(&mut this) {
        let mut widget: &dyn WidgetGroup = this;
        while let Some(parent) = widget.parent() {
            widget = parent;
            if !widget.is_scroll_pane() {
                continue;
            }

            let scroll_pane = widget.as_scroll_pane().unwrap();

            if let Some(listeners) = this.killed_listeners.remove(&scroll_pane) {
                for listener in listeners {
                    scroll_pane.add_listener(listener);
                }
            }

            if let Some(listeners) = this.killed_capture_listeners.remove(&scroll_pane) {
                for listener in listeners {
                    scroll_pane.add_capture_listener(listener);
                }
            }
        }
    }

    /// Shows the tip
    fn show_tip(&mut this) {
        if this.tip_container.has_parent() {
            return;
        }

        this.tip_container.pack();
        if this.needs_layout() {
            this.pack();
        }

        let pos = this.slider.local_to_parent_coordinates(Point2 {
            x: this.slider.width() / 2.0,
            y: this.slider.height(),
        });

        this.tip_container.set_origin(ggez::graphics::Align::Bottom);
        this.tip_container.set_position(pos.x, pos.y, ggez::graphics::Align::Bottom);
        this.tip_container.set_transform(true);
        this.tip_container.color_mut().a = 0.2;
        this.tip_container.set_scale(0.05);

        this.add_actor(this.tip_container.clone());

        this.tip_container.add_action(
            ggez::graphics::Action::parallel(
                ggez::graphics::Action::fade_in(Self::TIP_ANIMATION_DURATION, ggez::graphics::Interpolation::Fade),
                ggez::graphics::Action::scale_to(1.0, 1.0, 0.2, ggez::graphics::Interpolation::Fade),
            ),
        );
    }

    /// Hides the tip
    fn hide_tip(&mut this) {
        this.tip_container.add_action(
            ggez::graphics::Action::sequence(
                ggez::graphics::Action::parallel(
                    ggez::graphics::Action::alpha(0.2, 0.2, ggez::graphics::Interpolation::Fade),
                    ggez::graphics::Action::scale_to(0.05, 0.05, 0.2, ggez::graphics::Interpolation::Fade),
                ),
                ggez::graphics::Action::remove_actor(),
            ),
        );
    }
}

// Implement the necessary traits for UncivSlider
impl std::ops::Deref for UncivSlider {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &this.base
    }
}

impl std::ops::DerefMut for UncivSlider {
    fn deref_mut(&mut this) -> &mut Self::Target {
        &mut this.base
    }
}

impl Clone for UncivSlider {
    fn clone(&this) -> Self {
        Self {
            base: this.base.clone(),
            slider: this.slider.clone(),
            minus_button: this.minus_button.clone(),
            plus_button: this.plus_button.clone(),
            tip_label: this.tip_label.clone(),
            tip_container: this.tip_container.clone(),
            tip_hide_task: None, // Don't clone the timer
            snap_to_values: this.snap_to_values.clone(),
            snap_threshold: this.snap_threshold,
            tip_format: this.tip_format.clone(),
            has_focus: this.has_focus,
            block_listener: this.block_listener,
            tip_type: this.tip_type,
            get_tip_text: this.get_tip_text.clone(),
            on_change: this.on_change.clone(),
            killed_listeners: HashMap::new(), // Don't clone the listeners
            killed_capture_listeners: HashMap::new(), // Don't clone the listeners
        }
    }
}