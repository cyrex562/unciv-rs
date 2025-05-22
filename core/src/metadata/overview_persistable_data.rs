use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::ui::screens::overviewscreen::{EmpireOverviewCategories, EmpireOverviewTab};

/// Represents persistable data for the empire overview screen
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OverviewPersistableData {
    /// The map of categories to their persistable data
    #[serde(flatten)]
    map: HashMap<EmpireOverviewCategories, EmpireOverviewTabPersistableData>,
    /// The last selected category
    pub last: EmpireOverviewCategories,
}

impl OverviewPersistableData {
    /// Creates a new OverviewPersistableData with the given map and last category
    pub fn new(
        map: HashMap<EmpireOverviewCategories, EmpireOverviewTabPersistableData>,
        last: EmpireOverviewCategories,
    ) -> Self {
        OverviewPersistableData { map, last }
    }

    /// Updates the persistable data with the given page objects
    pub fn update(&mut self, page_objects: &HashMap<EmpireOverviewCategories, &EmpireOverviewTab>) {
        for (category, page) in page_objects {
            self.map.insert(*category, page.persistable_data().clone());
        }
    }

    /// Serializes the data to a JSON value
    pub fn to_json(&self) -> JsonValue {
        let mut json = json!({});

        if self.last != EmpireOverviewCategories::Cities {
            json["last"] = json!(self.last.to_string());
        }

        for (category, data) in &self.map {
            if let Some(persist_data_class) = category.get_persist_data_class() {
                if !data.is_empty() {
                    json[category.to_string()] = serde_json::to_value(data).unwrap_or(json!({}));
                }
            }
        }

        json
    }

    /// Deserializes the data from a JSON value
    pub fn from_json(json: &JsonValue) -> Self {
        let mut data = OverviewPersistableData::default();

        if let Some(last) = json.get("last").and_then(|v| v.as_str()) {
            if let Some(category) = EmpireOverviewCategories::from_str(last) {
                data.last = category;
            }
        }

        if let Some(obj) = json.as_object() {
            for (key, value) in obj {
                if key == "last" {
                    continue;
                }

                if let Some(category) = EmpireOverviewCategories::from_str(key) {
                    if let Some(persist_data_class) = category.get_persist_data_class() {
                        if let Ok(persist_data) = serde_json::from_value(value.clone()) {
                            data.map.insert(category, persist_data);
                        }
                    }
                }
            }
        }

        data
    }
}

/// Represents persistable data for an empire overview tab
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EmpireOverviewTabPersistableData {
    /// The sorted by field
    pub sorted_by: Option<String>,
    /// The sort direction
    pub direction: Option<String>,
    /// Whether the view is vertical
    pub vertical: Option<bool>,
}

impl EmpireOverviewTabPersistableData {
    /// Creates a new EmpireOverviewTabPersistableData
    pub fn new() -> Self {
        EmpireOverviewTabPersistableData::default()
    }

    /// Checks if the data is empty
    pub fn is_empty(&self) -> bool {
        self.sorted_by.is_none() && self.direction.is_none() && self.vertical.is_none()
    }
}

/// Extension trait for EmpireOverviewCategories
pub trait EmpireOverviewCategoriesExt {
    /// Gets the persist data class for the category
    fn get_persist_data_class(&self) -> Option<&'static str>;
}

impl EmpireOverviewCategoriesExt for EmpireOverviewCategories {
    fn get_persist_data_class(&self) -> Option<&'static str> {
        match self {
            EmpireOverviewCategories::Cities => Some("EmpireOverviewTabPersistableData"),
            EmpireOverviewCategories::Resources => Some("EmpireOverviewTabPersistableData"),
            _ => None,
        }
    }
}

/// Extension trait for EmpireOverviewTab
pub trait EmpireOverviewTabExt {
    /// Gets the persistable data for the tab
    fn persistable_data(&self) -> &EmpireOverviewTabPersistableData;
}

impl EmpireOverviewTabExt for EmpireOverviewTab {
    fn persistable_data(&self) -> &EmpireOverviewTabPersistableData {
        &self.persistable_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overview_persistable_data_default() {
        let data = OverviewPersistableData::default();
        assert_eq!(data.last, EmpireOverviewCategories::Cities);
        assert!(data.map.is_empty());
    }

    #[test]
    fn test_overview_persistable_data_to_json() {
        let mut data = OverviewPersistableData::default();
        data.last = EmpireOverviewCategories::Resources;

        let mut map = HashMap::new();
        let mut persist_data = EmpireOverviewTabPersistableData::new();
        persist_data.sorted_by = Some("Population".to_string());
        persist_data.direction = Some("Descending".to_string());
        map.insert(EmpireOverviewCategories::Cities, persist_data);
        data.map = map;

        let json = data.to_json();
        assert_eq!(json["last"], "Resources");
        assert_eq!(json["Cities"]["sorted_by"], "Population");
        assert_eq!(json["Cities"]["direction"], "Descending");
    }

    #[test]
    fn test_overview_persistable_data_from_json() {
        let json = json!({
            "last": "Resources",
            "Cities": {
                "sorted_by": "Population",
                "direction": "Descending"
            }
        });

        let data = OverviewPersistableData::from_json(&json);
        assert_eq!(data.last, EmpireOverviewCategories::Resources);
        assert_eq!(data.map.len(), 1);

        let cities_data = data.map.get(&EmpireOverviewCategories::Cities).unwrap();
        assert_eq!(cities_data.sorted_by, Some("Population".to_string()));
        assert_eq!(cities_data.direction, Some("Descending".to_string()));
    }
}