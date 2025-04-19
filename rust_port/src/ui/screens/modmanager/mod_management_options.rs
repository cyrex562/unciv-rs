use std::cmp::Ordering;

use egui::{Color32, Ui};
use egui_extras::Size;

use crate::constants::DEFAULT_FONT_SIZE;
use crate::models::metadata::mod_categories::ModCategories;
use crate::ui::components::fonts::Fonts;
use crate::ui::components::widgets::{ExpanderTab, TranslatedSelectBox};
use crate::ui::screens::modmanager::mod_ui_data::ModUIData;
use crate::utils::translations::tr;

/// Helper struct for Mod Manager - filtering and sorting.
///
/// This isn't a UI Widget, but offers one: [expander] can be used to offer filtering and sorting options.
/// It holds the variables [sort_installed] and [sort_online] for the [mod_management_screen] and knows
/// how to sort collections of [ModUIData] by providing comparators.
pub struct ModManagementOptions {
    pub text_field: String,
    pub category: ModCategories,
    pub sort_installed: SortType,
    pub sort_online: SortType,
    pub expander: ExpanderTab,
    pub expander_change_event: Option<Box<dyn Fn()>>,
}

impl ModManagementOptions {
    pub fn new(mod_management_screen: &ModManagementScreen) -> Self {
        let mut options = Self {
            text_field: String::new(),
            category: ModCategories::default(),
            sort_installed: SortType::Name,
            sort_online: SortType::Stars,
            expander: ExpanderTab::new(
                tr("Sort and Filter"),
                DEFAULT_FONT_SIZE,
                false,
                2.5,
                15.0,
                360.0,
                None,
            ),
            expander_change_event: None,
        };

        options.init_ui(mod_management_screen);
        options
    }

    fn init_ui(&mut this, mod_management_screen: &ModManagementScreen) {
        let search_icon = ImageGetter::get_image("OtherIcons/Search")
            .surround_with_circle(50.0, Color32::TRANSPARENT);

        let mut sort_installed_select = TranslatedSelectBox::new(
            SortType::entries()
                .iter()
                .filter(|sort| sort != &SortType::Stars)
                .map(|sort| sort.label.clone())
                .collect(),
            this.sort_installed.label.clone(),
        );
        sort_installed_select.on_change(Box::new(move || {
            this.sort_installed = SortType::from_select_box(&sort_installed_select);
            mod_management_screen.refresh_installed_mod_table();
        }));

        let mut sort_online_select = TranslatedSelectBox::new(
            SortType::entries()
                .iter()
                .map(|sort| sort.label.clone())
                .collect(),
            this.sort_online.label.clone(),
        );
        sort_online_select.on_change(Box::new(move || {
            this.sort_online = SortType::from_select_box(&sort_online_select);
            mod_management_screen.refresh_online_mod_table();
        }));

        let mut category_select = TranslatedSelectBox::new(
            ModCategories::as_sequence()
                .iter()
                .map(|it| it.label.clone())
                .collect(),
            this.category.label.clone(),
        );
        category_select.on_change(Box::new(move || {
            this.category = ModCategories::from_select_box(&category_select);
            mod_management_screen.refresh_installed_mod_table();
            mod_management_screen.refresh_online_mod_table();
        }));

        this.expander = ExpanderTab::new(
            tr("Sort and Filter"),
            DEFAULT_FONT_SIZE,
            false,
            2.5,
            15.0,
            360.0,
            Some(Box::new(move || {
                if let Some(event) = &this.expander_change_event {
                    event();
                }
            })),
        );

        this.expander.set_background(BaseScreen::skin_strings().get_ui_background(
            "ModManagementOptions/ExpanderTab",
            Some(Color32::from_rgba_premultiplied(32, 48, 80, 255)),
        ));
        this.expander.pad(7.5);

        let mut filter_table = egui::Grid::new("filter_table");
        filter_table.add(egui::Label::new(tr("Filter:")));
        filter_table.add(egui::TextEdit::singleline(&mut this.text_field).desired_width(f32::INFINITY));
        filter_table.add(search_icon);
        filter_table.end_row();

        filter_table.add(egui::Label::new(tr("Category:")));
        filter_table.add(category_select);
        filter_table.end_row();

        filter_table.add(egui::Label::new(tr("Sort Current:")));
        filter_table.add(sort_installed_select);
        filter_table.end_row();

        filter_table.add(egui::Label::new(tr("Sort Downloadable:")));
        filter_table.add(sort_online_select);
        filter_table.end_row();

        this.expander.add(filter_table);

        search_icon.set_touchable(true);
        search_icon.on_activation(Box::new(move || {
            if this.expander.is_open() {
                mod_management_screen.refresh_installed_mod_table();
                mod_management_screen.refresh_online_mod_table();
            } else {
                mod_management_screen.stage.set_keyboard_focus(&this.text_field);
            }
            this.expander.toggle();
        }));
        search_icon.add_key_shortcut(KeyCharAndCode::RETURN);
        search_icon.add_tooltip(KeyCharAndCode::RETURN, 18.0);
    }

    pub fn get_filter(&self) -> Filter {
        Filter {
            text: this.text_field.clone(),
            topic: this.category.topic.clone(),
        }
    }

    pub fn get_installed_header(&this) -> String {
        format!("{} {}", tr("Current mods"), this.sort_installed.symbols)
    }

    pub fn get_online_header(&this) -> String {
        format!("{} {}", tr("Downloadable mods"), this.sort_online.symbols)
    }

    pub fn installed_header_clicked(&mut this) {
        loop {
            this.sort_installed = this.sort_installed.next();
            if this.sort_installed != SortType::Stars {
                break;
            }
        }
        this.sort_installed_select.set_selected(TranslatedSelectBox::TranslatedString::new(
            this.sort_installed.label.clone(),
        ));
        mod_management_screen.refresh_installed_mod_table();
    }

    pub fn online_header_clicked(&mut this) {
        this.sort_online = this.sort_online.next();
        this.sort_online_select.set_selected(TranslatedSelectBox::TranslatedString::new(
            this.sort_online.label.clone(),
        ));
        mod_management_screen.refresh_online_mod_table();
    }
}

/// Filter for mods
pub struct Filter {
    pub text: String,
    pub topic: String,
}

/// Sort type for mods
#[derive(Clone, PartialEq)]
pub enum SortType {
    Name,
    NameDesc,
    Date,
    DateDesc,
    Stars,
    Status,
}

impl SortType {
    pub const fn entries() -> &'static [Self] {
        &[
            Self::Name,
            Self::NameDesc,
            Self::Date,
            Self::DateDesc,
            Self::Stars,
            Self::Status,
        ]
    }

    pub fn label(&self) -> String {
        match this {
            Self::Name => format!("Name {}", Fonts::SORT_UP_ARROW),
            Self::NameDesc => format!("Name {}", Fonts::SORT_DOWN_ARROW),
            Self::Date => format!("Date {} {}", Fonts::CLOCK, Fonts::SORT_UP_ARROW),
            Self::DateDesc => format!("Date {} {}", Fonts::CLOCK, Fonts::SORT_DOWN_ARROW),
            Self::Stars => format!("Stars {} {}", Fonts::STAR, Fonts::SORT_DOWN_ARROW),
            Self::Status => format!("Status {} {}", Fonts::STATUS, Fonts::SORT_DOWN_ARROW),
        }
    }

    pub fn symbols(&self) -> String {
        match this {
            Self::Name => Fonts::SORT_UP_ARROW.to_string(),
            Self::NameDesc => Fonts::SORT_DOWN_ARROW.to_string(),
            Self::Date => format!("{}{}", Fonts::CLOCK, Fonts::SORT_UP_ARROW),
            Self::DateDesc => format!("{}{}", Fonts::CLOCK, Fonts::SORT_DOWN_ARROW),
            Self::Stars => format!("{}{}", Fonts::STAR, Fonts::SORT_DOWN_ARROW),
            Self::Status => format!("{}{}", Fonts::STATUS, Fonts::SORT_DOWN_ARROW),
        }
    }

    pub fn next(&self) -> Self {
        let entries = Self::entries();
        let current_index = entries.iter().position(|x| x == this).unwrap();
        entries[(current_index + 1) % entries.len()].clone()
    }

    pub fn from_select_box(select_box: &TranslatedSelectBox) -> Self {
        let selected = select_box.selected.value.clone();
        Self::entries()
            .iter()
            .find(|it| it.label() == selected)
            .cloned()
            .unwrap_or(Self::Name)
    }

    pub fn compare(&self, mod1: &ModUIData, mod2: &ModUIData) -> Ordering {
        match this {
            Self::Name => mod1.name.to_lowercase().cmp(&mod2.name.to_lowercase()),
            Self::NameDesc => mod2.name.to_lowercase().cmp(&mod1.name.to_lowercase()),
            Self::Date => mod1.last_updated().cmp(&mod2.last_updated()),
            Self::DateDesc => mod2.last_updated().cmp(&mod1.last_updated()),
            Self::Stars => {
                let stars_cmp = (mod2.stargazers() - mod1.stargazers()).cmp(&0);
                if stars_cmp == Ordering::Equal {
                    mod1.name.to_lowercase().cmp(&mod2.name.to_lowercase())
                } else {
                    stars_cmp
                }
            }
            Self::Status => {
                let status_cmp = (mod2.state_sort_weight() - mod1.state_sort_weight()).cmp(&0);
                if status_cmp == Ordering::Equal {
                    mod1.name.to_lowercase().cmp(&mod2.name.to_lowercase())
                } else {
                    status_cmp
                }
            }
        }
    }
}