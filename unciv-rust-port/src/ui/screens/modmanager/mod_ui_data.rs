use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::logic::github::github_api::Repo;
use crate::models::metadata::mod_categories::ModCategories;
use crate::models::ruleset::ruleset::Ruleset;
use crate::ui::components::fonts::Fonts;
use crate::ui::screens::modmanager::mod_management_options::Filter;
use crate::utils::translations::tr;

/// Helper struct holds combined mod info for ModManagementScreen, used for both installed and online lists.
///
/// Contains metadata only, some preformatted for the UI, but no UI components!
/// (This is important on resize - ModUIData are passed to the new screen)
/// Note it is guaranteed either ruleset or repo are non-null, never both.
pub struct ModUIData {
    pub name: String,
    pub description: String,
    pub ruleset: Option<Ruleset>,
    pub repo: Option<Repo>,
    pub is_visual: bool,
    pub has_update: bool,
}

impl ModUIData {
    /// Default constructor for deserialization from cache file
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            ruleset: None,
            repo: None,
            is_visual: false,
            has_update: false,
        }
    }

    /// Constructor for installed mods
    pub fn from_ruleset(ruleset: Ruleset, is_visual: bool) -> Self {
        let summary = ruleset.get_summary();
        let description = if summary.is_empty() {
            format!("{}", tr("Installed"))
        } else {
            format!("{}: {}", tr("Installed"), summary)
        };

        Self {
            name: ruleset.name.clone(),
            description,
            ruleset: Some(ruleset),
            repo: None,
            is_visual,
            has_update: false,
        }
    }

    /// Constructor for online mods
    pub fn from_repo(repo: Repo, is_updated: bool) -> Self {
        let description = if let Some(desc) = &repo.description {
            format!("{}\n[{}]{}", desc, repo.stargazers_count, Fonts::STAR)
        } else {
            format!("-{}-", tr("No description provided"))
        };

        Self {
            name: repo.name.clone(),
            description,
            ruleset: None,
            repo: Some(repo),
            is_visual: false,
            has_update: is_updated,
        }
    }

    /// Whether this mod is installed locally
    pub fn is_installed(&self) -> bool {
        self.ruleset.is_some()
    }

    /// Get the last update timestamp
    pub fn last_updated(&self) -> String {
        if let Some(ruleset) = &self.ruleset {
            if let Some(mod_options) = &ruleset.mod_options {
                return mod_options.last_updated.clone();
            }
        }
        if let Some(repo) = &self.repo {
            return repo.pushed_at.clone();
        }
        String::new()
    }

    /// Get the number of stargazers for this mod
    pub fn stargazers(&self) -> i32 {
        self.repo.as_ref().map(|r| r.stargazers_count).unwrap_or(0)
    }

    /// Get the author of this mod
    pub fn author(&self) -> String {
        if let Some(ruleset) = &self.ruleset {
            if let Some(mod_options) = &ruleset.mod_options {
                return mod_options.author.clone();
            }
        }
        if let Some(repo) = &self.repo {
            if let Some(owner) = &repo.owner {
                return owner.login.clone();
            }
        }
        String::new()
    }

    /// Get the topics/categories for this mod
    pub fn topics(&self) -> Vec<String> {
        if let Some(repo) = &self.repo {
            return repo.topics.clone();
        }
        if let Some(ruleset) = &self.ruleset {
            if let Some(mod_options) = &ruleset.mod_options {
                return mod_options.topics.clone();
            }
        }
        Vec::new()
    }

    /// Get the text to display on the mod button
    pub fn button_text(&self) -> String {
        match (&self.ruleset, &self.repo) {
            (Some(ruleset), None) => ruleset.name.clone(),
            (None, Some(repo)) => {
                if self.has_update {
                    format!("{} - {}", repo.name, tr("Updated"))
                } else {
                    repo.name.clone()
                }
            }
            _ => String::new(),
        }
    }

    /// Check if this mod matches the given filter
    pub fn matches_filter(&self, filter: &Filter) -> bool {
        if !self.matches_category(filter) {
            return false;
        }
        if filter.text.is_empty() {
            return true;
        }
        if self.name.to_lowercase().contains(&filter.text.to_lowercase()) {
            return true;
        }
        if self.author().to_lowercase().contains(&filter.text.to_lowercase()) {
            return true;
        }
        false
    }

    /// Check if this mod matches the given category filter
    fn matches_category(&self, filter: &Filter) -> bool {
        if filter.topic == ModCategories::default().topic {
            return true;
        }
        let mod_topics = match (&self.repo, &self.ruleset) {
            (Some(repo), _) => repo.topics.clone(),
            (None, Some(ruleset)) => {
                if let Some(mod_options) = &ruleset.mod_options {
                    mod_options.topics.clone()
                } else {
                    return false;
                }
            }
            _ => return false,
        };
        mod_topics.contains(&filter.topic)
    }

    /// Get the sort weight based on mod state
    pub fn state_sort_weight(&self) -> i32 {
        match (self.has_update, self.is_visual) {
            (true, true) => 3,
            (true, false) => 2,
            (false, true) => 1,
            (false, false) => 0,
        }
    }
}

impl Hash for ModUIData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        (if self.is_installed() { 31 } else { 19 }).hash(state);
    }
}

impl PartialEq for ModUIData {
    fn eq(&self, other: &Self) -> bool {
        self.is_installed() == other.is_installed() && self.name == other.name
    }
}

impl Eq for ModUIData {}