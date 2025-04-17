use std::rc::Rc;
use std::time::Duration;
use ggez::graphics::Align;
use ggez::mint::Point2;

use crate::ui::components::color_markup_label::ColorMarkupLabel;
use crate::ui::screens::base_screen::BaseScreen;
use crate::utils::concurrency::Concurrency;
use crate::utils::launch_on_gl_thread;

use super::popup::Popup;

/// This is an unobtrusive popup which will close itself after a given amount of time.
/// - Will show on top of other Popups, but not on top of other ToastPopups
/// - Several calls in a short time will be shown sequentially, each "waiting their turn"
/// - The user can close a Toast by clicking it
/// - Supports color markup via ColorMarkupLabel, using «» instead of Gdx's [].
pub struct ToastPopup {
    /// The base popup
    base: Popup,
    /// The message to display
    message: String,
    /// The duration in milliseconds before auto-closing
    time: u64,
    /// Whether the timer has been started
    timer_started: bool,
}

impl ToastPopup {
    /// Creates a new ToastPopup with the given message, screen, and time
    ///
    /// # Arguments
    ///
    /// * `message` - The message to display
    /// * `screen` - The screen to show the popup on
    /// * `time` - Duration in milliseconds, defaults to 2 seconds
    pub fn new(message: String, screen: &Rc<BaseScreen>, time: u64) -> Self {
        let mut popup = Self {
            base: Popup::new_with_screen(screen, super::popup::Scrollability::None, 0.9),
            message,
            time,
            timer_started: false,
        };

        // Make this popup unobtrusive
        popup.set_fill_parent(false);

        // Add click handler to close the popup
        popup.on_click(Box::new(move || {
            popup.close();
        }));

        // Add the message label
        let mut label = ColorMarkupLabel::new(&popup.message);
        label.set_wrap(true);
        label.set_alignment(Align::Center);
        popup.add(label);
        popup.width(screen.width() / 2.0);

        // Open the popup if no other toast popups are visible
        let force = !screen.has_toast_popups();
        popup.open(force);

        // Move it to the top so it's not in the middle of the screen
        // Has to be done after open() because open() centers the popup
        let y = screen.height() - (popup.height() + 20.0);
        popup.set_position(popup.position().x, y);

        popup
    }

    /// Creates a new ToastPopup with the default time of 2 seconds
    pub fn new_with_default_time(message: String, screen: &Rc<BaseScreen>) -> Self {
        Self::new(message, screen, 2000)
    }

    /// Starts the timer to close the popup after the specified time
    fn start_timer(&mut self) {
        if self.timer_started {
            return;
        }

        self.timer_started = true;
        let time = self.time;
        let popup = self.clone();

        Concurrency::run("ToastPopup", move || {
            std::thread::sleep(Duration::from_millis(time));
            launch_on_gl_thread(move || {
                popup.close();
            });
        });
    }
}

impl Clone for ToastPopup {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            message: self.message.clone(),
            time: self.time,
            timer_started: false, // Reset timer_started on clone
        }
    }
}

impl std::ops::Deref for ToastPopup {
    type Target = Popup;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for ToastPopup {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Extension trait for BaseScreen to add toast popup functionality
pub trait ToastPopupExt {
    /// Returns a list of currently active ToastPopups
    fn toast_popups(&self) -> Vec<&ToastPopup>;

    /// Checks if there are visible ToastPopups
    fn has_toast_popups(&self) -> bool;
}

impl ToastPopupExt for BaseScreen {
    fn toast_popups(&self) -> Vec<&ToastPopup> {
        self.widgets()
            .iter()
            .filter_map(|w| w.downcast_ref::<ToastPopup>())
            .collect()
    }

    fn has_toast_popups(&self) -> bool {
        self.toast_popups().iter().any(|p| p.is_visible())
    }
}

/// Override the set_visible method to start the timer when the popup becomes visible
impl ToastPopup {
    /// Sets the visibility of the popup and starts the timer if it becomes visible
    pub fn set_visible(&mut self, visible: bool) {
        if visible {
            self.start_timer();
        }
        self.base.set_visible(visible);
    }
}