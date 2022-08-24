use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use levenshtein::levenshtein;
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    pub name: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub yesterday_vol: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub today_vol: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub custom_avg: f32,
}

pub struct Database {
    items: Vec<Item>,
}

impl Database {
    pub fn load_from_file(file: Option<&Path>) -> Database {
        // download file from: https://api.warframestat.us/wfinfo/prices
        let text = read_to_string(file.unwrap_or_else(|| Path::new("prices.json"))).unwrap();
        let mut items: Vec<Item> = serde_json::from_str(&text).unwrap();

        items.iter_mut().for_each(|item| {
            item.name = item
                .name
                .replace("Systems", "Systems Blueprint")
                .replace("Neuroptics", "Neuroptics Blueprint")
                .replace("Chassis", "Chassis Blueprint")
        });

        Database { items }
    }

    pub fn find_item(&self, needle: &str, threshold: Option<usize>) -> Option<&Item> {
        let best_match = self.items.iter().min_by_key(|item| {
            #[cfg(test)]
            println!(
                "{} {} -> {}",
                item.name,
                needle,
                levenshtein(&item.name, needle)
            );

            levenshtein(&item.name, needle)
        });

        best_match.and_then(|item| {
            if levenshtein(&item.name, needle) <= threshold.unwrap_or(item.name.len() / 3) {
                Some(item)
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn can_load_database() {
        Database::load_from_file(None);
    }

    #[test]
    pub fn can_find_items() {
        let db = Database::load_from_file(None);

        let item = db
            .find_item("Titania Prime Blueprint", Some(0))
            .expect("Failed to find Titania Prime Blueprint in database");
        assert_eq!(item.name, "Titania Prime Blueprint");

        let item = db
            .find_item("Octavia Prime Blueprint", Some(0))
            .expect("Failed to find Octavia Prime Blueprint in database");
        assert_eq!(item.name, "Octavia Prime Blueprint");
    }

    #[test]
    pub fn can_find_fuzzy_items() {
        let db = Database::load_from_file(None);

        let item = db
            .find_item("Akstlett Prlme Recver", None)
            .expect("Failed to fuzzy find Akstiletto Prime Receiver in database");
        assert_eq!(item.name, "Akstiletto Prime Receiver");

        let item = db
            .find_item("ctavio Prlme Blueprnt", None)
            .expect("Failed to fuzzy find Octavia Prime Blueprint in database");
        assert_eq!(item.name, "Octavia Prime Blueprint");

        let item = db
            .find_item("Oclavia Prime Syslems\nBlueprint\n", None)
            .expect("Failed to fuzzy find Octavia Prime Blueprint in database");
        assert_eq!(item.name, "Octavia Prime Systems Blueprint");
    }
}
