use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::logic::github::{GithubAPI, Github};
use crate::ui::components::widgets::TranslatedSelectBox;

/// Represents a category of mods
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Category {
    /// The display label for the category
    pub label: String,
    /// The topic name for the category
    pub topic: String,
    /// Whether the category is hidden
    pub hidden: bool,
    /// Copy of github created_at, no function except help evaluate
    #[serde(skip_serializing_if = "String::is_empty")]
    pub create_date: String,
    /// Copy of github updated_at, no function except help evaluate
    pub modify_date: String,
}

impl Default for Category {
    fn default() -> Self {
        Category {
            label: String::new(),
            topic: String::new(),
            hidden: false,
            create_date: String::new(),
            modify_date: String::new(),
        }
    }
}

impl Category {
    /// Creates a new Category with the given parameters
    pub fn new(label: String, topic: String, hidden: bool, create_date: String, modify_date: String) -> Self {
        Category {
            label,
            topic,
            hidden,
            create_date,
            modify_date,
        }
    }

    /// Creates a new Category from a Github topic
    pub fn from_topic(topic: &GithubAPI::TopicSearchResponse::Topic) -> Self {
        Category {
            label: Self::label_suggestion(topic),
            topic: topic.name.clone(),
            hidden: true,
            create_date: topic.created_at.clone(),
            modify_date: topic.updated_at.clone(),
        }
    }

    /// Gets a label suggestion for a topic
    pub fn label_suggestion(topic: &GithubAPI::TopicSearchResponse::Topic) -> String {
        if let Some(display_name) = &topic.display_name {
            if !display_name.is_empty() {
                return display_name.clone();
            }
        }

        let name = topic.name.clone();
        if name.starts_with("unciv-mod-") {
            let name = name.trim_start_matches("unciv-mod-");
            if !name.is_empty() {
                let mut chars: Vec<char> = name.chars().collect();
                if !chars.is_empty() {
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                }
                return chars.into_iter().collect();
            }
        }

        name
    }

    /// The "All mods" category
    pub fn all() -> Self {
        Category::new(
            "All mods".to_string(),
            "unciv-mod".to_string(),
            false,
            String::new(),
            String::new(),
        )
    }
}

/// A collection of mod categories
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModCategories {
    /// The categories in this collection
    pub categories: Vec<Category>,
}

impl Default for ModCategories {
    fn default() -> Self {
        ModCategories {
            categories: vec![Category::all()],
        }
    }
}

impl ModCategories {
    /// The file location for the mod categories
    const FILE_LOCATION: &'static str = "jsons/ModCategories.json";

    /// Creates a new ModCategories instance
    pub fn new() -> Self {
        ModCategories::default()
    }

    /// Loads the mod categories from the file
    pub fn load() -> Self {
        let path = Path::new(Self::FILE_LOCATION);
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(contents) => {
                    match serde_json::from_str(&contents) {
                        Ok(categories) => categories,
                        Err(_) => ModCategories::default(),
                    }
                }
                Err(_) => ModCategories::default(),
            }
        } else {
            ModCategories::default()
        }
    }

    /// Saves the mod categories to the file
    fn save(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(Self::FILE_LOCATION, json)?;
        Ok(())
    }

    /// Gets a category from a select box
    pub fn from_select_box(&self, select_box: &TranslatedSelectBox) -> Category {
        let selected = select_box.selected();
        self.categories.iter()
            .find(|c| c.label == selected)
            .cloned()
            .unwrap_or_else(Category::all)
    }

    /// Merges the categories with online categories
    pub fn merge_online(&mut self) -> String {
        match Github::try_get_github_topics() {
            Some(topics) => {
                let mut new_count = 0;
                let mut sorted_topics: Vec<_> = topics.items.iter().collect();
                sorted_topics.sort_by(|a, b| a.name.cmp(&b.name));

                for topic in sorted_topics {
                    if let Some(existing) = self.categories.iter_mut().find(|c| c.topic == topic.name) {
                        existing.modify_date = topic.updated_at.clone();
                    } else {
                        self.categories.push(Category::from_topic(topic));
                        new_count += 1;
                    }
                }

                if let Err(e) = self.save() {
                    return format!("Failed to save: {}", e);
                }

                format!("{} new categories", new_count)
            }
            None => "Failed".to_string(),
        }
    }

    /// Gets the categories as an iterator, filtering out hidden categories
    pub fn visible_categories(&self) -> impl Iterator<Item = &Category> {
        self.categories.iter().filter(|c| !c.hidden)
    }
}

impl IntoIterator for ModCategories {
    type Item = Category;
    type IntoIter = std::vec::IntoIter<Category>;

    fn into_iter(self) -> Self::IntoIter {
        self.categories.into_iter().filter(|c| !c.hidden).collect::<Vec<_>>().into_iter()
    }
}

/// A singleton instance of ModCategories
lazy_static::lazy_static! {
    static ref INSTANCE: ModCategories = ModCategories::load();
}

impl ModCategories {
    /// Gets the default category
    pub fn default_category() -> Category {
        Category::all()
    }

    /// Merges the categories with online categories
    pub fn merge_online() -> String {
        let mut instance = INSTANCE.clone();
        instance.merge_online()
    }

    /// Gets a category from a select box
    pub fn from_select_box(select_box: &TranslatedSelectBox) -> Category {
        INSTANCE.from_select_box(select_box)
    }

    /// Gets the visible categories as an iterator
    pub fn visible_categories() -> impl Iterator<Item = &'static Category> {
        INSTANCE.visible_categories()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_all() {
        let category = Category::all();
        assert_eq!(category.label, "All mods");
        assert_eq!(category.topic, "unciv-mod");
        assert!(!category.hidden);
    }

    #[test]
    fn test_label_suggestion() {
        let topic = GithubAPI::TopicSearchResponse::Topic {
            name: "unciv-mod-test".to_string(),
            display_name: None,
            created_at: String::new(),
            updated_at: String::new(),
        };

        assert_eq!(Category::label_suggestion(&topic), "Test");

        let topic = GithubAPI::TopicSearchResponse::Topic {
            name: "test".to_string(),
            display_name: None,
            created_at: String::new(),
            updated_at: String::new(),
        };

        assert_eq!(Category::label_suggestion(&topic), "test");

        let topic = GithubAPI::TopicSearchResponse::Topic {
            name: "test".to_string(),
            display_name: Some("Test Display".to_string()),
            created_at: String::new(),
            updated_at: String::new(),
        };

        assert_eq!(Category::label_suggestion(&topic), "Test Display");
    }

    #[test]
    fn test_mod_categories_default() {
        let categories = ModCategories::default();
        assert_eq!(categories.categories.len(), 1);
        assert_eq!(categories.categories[0].label, "All mods");
    }

    #[test]
    fn test_mod_categories_from_select_box() {
        let categories = ModCategories::default();
        let select_box = TranslatedSelectBox::new();
        select_box.set_selected("All mods".to_string());

        let category = categories.from_select_box(&select_box);
        assert_eq!(category.label, "All mods");
    }
}