use std::sync::Arc;
use std::time::Duration;

use ggez::graphics::Color;
use ggez::mint::Point2;
use ggez::timer::Timer;

use crate::ui::components::widgets::text_field::TextField;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::components::widgets::widget_group::WidgetGroup;
use crate::ui::components::widgets::scroll_pane::ScrollPane;
use crate::ui::components::widgets::label::Label;
use crate::ui::components::widgets::popup::Popup;
use crate::ui::components::extensions::get_ascendant;
use crate::ui::components::extensions::get_overlap;
use crate::ui::components::extensions::right;
use crate::ui::components::extensions::stage_bounding_box;
use crate::ui::components::extensions::top;
use crate::ui::components::input::key_shortcuts;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::basescreen::UncivStage;
use crate::ui::translations::TranslationManager;
use crate::ui::utils::concurrency::Concurrency;
use crate::ui::utils::with_gl_context;
use crate::ui::constants::Constants;
use crate::ui::event_bus::EventBus;
use crate::ui::event_bus::EventReceiver;

/// A text field with platform-specific input handling
pub struct UncivTextField {
    /// The base TextField that this UncivTextField extends
    base: TextField,

    /// Whether this is running on Android
    is_android: bool,

    /// The function to hide the keyboard
    hide_keyboard: Arc<dyn Fn() + Send + Sync>,

    /// The on focus change callback
    on_focus_change: Option<Arc<dyn Fn(&mut UncivTextField, bool) + Send + Sync>>,

    /// The focus listener
    focus_listener: UncivTextFieldFocusListener,

    /// The visible area changed listener
    visible_area_changed_listener: Option<VisibleAreaChangedListener>,
}

/// The focus listener for UncivTextField
struct UncivTextFieldFocusListener {
    /// The parent UncivTextField
    parent: Arc<UncivTextField>,
}

impl UncivTextFieldFocusListener {
    /// Creates a new UncivTextFieldFocusListener
    fn new(parent: Arc<UncivTextField>) -> Self {
        Self { parent }
    }

    /// Called when keyboard focus changes
    fn keyboard_focus_changed(&self, event: &ggez::event::FocusEvent, actor: &dyn Widget, focused: bool) {
        if focused {
            self.parent.scroll_ascendant_to_text_field();
            if self.parent.is_android {
                self.parent.add_popup_close_listener();
                // Show on-screen keyboard
                ggez::input::set_onscreen_keyboard_visible(true);
            }
        }

        if let Some(on_focus_change) = &self.parent.on_focus_change {
            on_focus_change(&mut *Arc::get_mut(&mut self.parent.clone()).unwrap(), focused);
        }
    }
}

/// The visible area changed listener for UncivTextField
struct VisibleAreaChangedListener {
    /// The parent UncivTextField
    parent: Arc<UncivTextField>,

    /// The event receiver
    events: EventReceiver,
}

impl VisibleAreaChangedListener {
    /// Creates a new VisibleAreaChangedListener
    fn new(parent: Arc<UncivTextField>) -> Self {
        let mut listener = Self {
            parent,
            events: EventReceiver::new(),
        };

        // Set up event handling
        listener.events.receive::<UncivStage::VisibleAreaChanged>(Box::new(move |_| {
            if listener.parent.stage().is_none() || !listener.parent.has_keyboard_focus() {
                return;
            }

            Concurrency::run(Box::new(move || {
                // If anything resizes, it also does so with this event. So we need to wait for that to finish to update the scroll position.
                std::thread::sleep(Duration::from_millis(100));

                with_gl_context(Box::new(move || {
                    if listener.parent.stage().is_none() {
                        return;
                    }

                    if listener.parent.scroll_ascendant_to_text_field() {
                        let scroll_pane = listener.parent.get_ascendant::<ScrollPane>();
                        // when screen dimensions change, we don't want an animation for scrolling, just show the textfield immediately
                        if let Some(scroll_pane) = scroll_pane {
                            scroll_pane.update_visual_scroll();
                        }
                    } else {
                        // We can't scroll the text field into view, so we need to show a popup
                        let popup = TextfieldPopup::new(listener.parent.clone());
                        popup.open();
                    }
                }));
            }));
        }));

        listener
    }

    /// Called when a touch down event occurs
    fn touch_down(&self, event: &ggez::event::TouchInputEvent, x: f32, y: f32, pointer: i32, button: i32) -> bool {
        self.parent.add_popup_close_listener();
        false
    }
}

/// The popup for UncivTextField
struct TextfieldPopup {
    /// The base Popup that this TextfieldPopup extends
    base: Popup,

    /// The popup text field
    popup_textfield: TextField,

    /// The parent UncivTextField
    parent: Arc<UncivTextField>,
}

impl TextfieldPopup {
    /// Creates a new TextfieldPopup
    fn new(parent: Arc<UncivTextField>) -> Self {
        let stage = parent.stage().unwrap();
        let mut popup = Self {
            base: Popup::new(stage.clone()),
            popup_textfield: TextField::new(parent.text().to_string(), BaseScreen::skin()),
            parent,
        };

        // Set up the popup
        popup.base.add_good_sized_label(popup.popup_textfield.message_text().to_string())
            .colspan(2)
            .row();

        popup.base.add(popup.popup_textfield.clone())
            .width(stage.width() / 2.0)
            .colspan(2)
            .row();

        popup.base.add_close_button(Constants::CANCEL.to_string())
            .left();

        popup.base.add_ok_button(Box::new(move || {
            popup.parent.copy_text_and_selection(&popup.popup_textfield);
        }))
            .right()
            .row();

        // Add show and close listeners
        popup.base.add_show_listener(Box::new(move || {
            stage.set_keyboard_focus(Some(popup.popup_textfield.clone()));
        }));

        popup.base.add_close_listener(Box::new(move || {
            stage.set_keyboard_focus(None);
            ggez::input::set_onscreen_keyboard_visible(false);
        }));

        popup
    }

    /// Opens the popup
    fn open(&self) {
        self.base.open();
    }
}

impl UncivTextField {
    /// Creates a new UncivTextField with the given parameters
    pub fn new(
        hint: String,
        pre_entered_text: String,
        on_focus_change: Option<Arc<dyn Fn(&mut UncivTextField, bool) + Send + Sync>>,
    ) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        let is_android = ggez::app::platform() == ggez::app::Platform::Android;
        let hide_keyboard = Arc::new(move || {
            ggez::input::set_onscreen_keyboard_visible(false);
        });

        let mut text_field = Self {
            base: TextField::new(pre_entered_text, skin),
            is_android,
            hide_keyboard,
            on_focus_change,
            focus_listener: UncivTextFieldFocusListener::new(Arc::new(text_field.clone())),
            visible_area_changed_listener: None,
        };

        // Set the message text
        text_field.base.set_message_text(hint.tr());

        // Add the focus listener
        text_field.base.add_focus_listener(Box::new(move |event, actor, focused| {
            text_field.focus_listener.keyboard_focus_changed(event, actor, focused);
        }));

        // Add the visible area changed listener if on Android
        if text_field.is_android {
            text_field.visible_area_changed_listener = Some(VisibleAreaChangedListener::new(Arc::new(text_field.clone())));
        }

        text_field
    }

    /// Adds a popup close listener
    fn add_popup_close_listener(&self) {
        let popup = self.get_ascendant::<Popup>();
        if let Some(popup) = popup {
            if !popup.close_listeners().contains(&self.hide_keyboard) {
                popup.add_close_listener(self.hide_keyboard.clone());
            }
        }
    }

    /// Tries to scroll a ScrollPane ascendant of the text field so that this text field is in the middle of the visible area.
    ///
    /// Returns true if the text field is visible after this operation
    fn scroll_ascendant_to_text_field(&self) -> bool {
        let stage = self.stage().unwrap().as_any().downcast_ref::<UncivStage>().unwrap();

        let scroll_pane = self.get_ascendant::<ScrollPane>();
        let visible_area = stage.last_known_visible_area();
        let text_field_stage_bounding_box = self.stage_bounding_box();

        if scroll_pane.is_none() {
            return visible_area.contains(text_field_stage_bounding_box);
        }

        let scroll_pane = scroll_pane.unwrap();
        let scroll_pane_bounds = scroll_pane.stage_bounding_box();
        let visible_scroll_pane_area = scroll_pane_bounds.get_overlap(visible_area);

        if visible_scroll_pane_area.is_none() {
            return false;
        }

        let visible_scroll_pane_area = visible_scroll_pane_area.unwrap();

        if visible_scroll_pane_area.contains(text_field_stage_bounding_box) {
            return true;
        }

        let scroll_content = scroll_pane.actor();
        let text_field_scroll_content_coords = self.local_to_ascendant_coordinates(scroll_content, Point2 { x: 0.0, y: 0.0 });

        // It's possible that our textField can't be (fully) scrolled to be within the visible scrollPane area
        let pixels_not_visible_on_left_side = (visible_scroll_pane_area.x - scroll_pane_bounds.x).max(0.0);
        let text_field_distance_from_left_side = text_field_scroll_content_coords.x;
        let pixels_not_visible_on_right_side = (scroll_pane_bounds.right() - visible_scroll_pane_area.right()).max(0.0);
        let text_field_distance_from_right_side = scroll_content.width() - (text_field_scroll_content_coords.x + self.width());
        let pixels_not_visible_on_top = (scroll_pane_bounds.top() - visible_scroll_pane_area.top()).max(0.0);
        let text_field_distance_from_top = scroll_content.height() - (text_field_scroll_content_coords.y + self.height());
        let pixels_not_visible_on_bottom = (visible_scroll_pane_area.y - scroll_pane_bounds.y).max(0.0);
        let text_field_distance_from_bottom = text_field_scroll_content_coords.y;

        // If the visible scroll pane area is smaller than our text field, it will always be partly obscured
        if visible_scroll_pane_area.width() < self.width() || visible_scroll_pane_area.height() < self.height()
            // If the amount of pixels obscured near a scrollContent edge is larger than the distance of the text field to that edge, it will always be (partly) obscured
            || pixels_not_visible_on_left_side > text_field_distance_from_left_side
            || pixels_not_visible_on_right_side > text_field_distance_from_right_side
            || pixels_not_visible_on_top > text_field_distance_from_top
            || pixels_not_visible_on_bottom > text_field_distance_from_bottom {
            return false;
        }

        // We want to put the text field in the middle of the visible area
        let scroll_x_middle = text_field_scroll_content_coords.x - self.width() / 2.0 + visible_scroll_pane_area.width() / 2.0;
        // If the visible area is to the right of the left edge of the scroll pane, we need to scroll that much farther to get to the real visible middle
        scroll_pane.set_scroll_x(pixels_not_visible_on_left_side + scroll_x_middle);

        // ScrollPane.scrollY has the origin at the top instead of at the bottom, so + for height / 2 instead of -
        // We want to put the text field in the middle of the visible area
        let scroll_y_middle_gdx_origin = text_field_scroll_content_coords.y + self.height() / 2.0 + visible_scroll_pane_area.height() / 2.0;
        // If the visible area is below the top edge of the scroll pane, we need to scroll that much farther to get to the real visible middle
        // Also, convert to scroll pane origin (0 is on top instead of bottom)
        scroll_pane.set_scroll_y(pixels_not_visible_on_top + scroll_content.height() - scroll_y_middle_gdx_origin);

        true
    }

    /// Copies text and selection from another text field
    fn copy_text_and_selection(&mut self, other: &TextField) {
        self.base.set_text(other.text().to_string());
        self.base.set_selection(other.selection_start(), other.selection_end());
    }
}

// Implement the necessary traits for UncivTextField
impl std::ops::Deref for UncivTextField {
    type Target = TextField;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for UncivTextField {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Clone for UncivTextField {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            is_android: self.is_android,
            hide_keyboard: self.hide_keyboard.clone(),
            on_focus_change: self.on_focus_change.clone(),
            focus_listener: UncivTextFieldFocusListener::new(Arc::new(self.clone())),
            visible_area_changed_listener: self.visible_area_changed_listener.as_ref().map(|listener| {
                VisibleAreaChangedListener::new(Arc::new(self.clone()))
            }),
        }
    }
}