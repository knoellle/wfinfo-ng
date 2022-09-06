use std::{collections::HashMap, fs::read_to_string, path::Path};

use levenshtein::levenshtein;
use serde::Deserialize;

use crate::wfinfo_data::{
    item_data::{EquipmentType, FilteredItems},
    price_data::PriceItem,
};

#[derive(Clone, Debug, Deserialize)]
pub struct Database {
    items: Vec<Item>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    pub name: String,
    pub platinum: f32,
    pub ducats: usize,
}

impl Database {
    pub fn load_from_file(prices: Option<&Path>, filtered_items: Option<&Path>) -> Database {
        // download file from: https://api.warframestat.us/wfinfo/prices
        let text = read_to_string(prices.unwrap_or_else(|| Path::new("prices.json"))).unwrap();
        let price_list: Vec<PriceItem> = serde_json::from_str(&text).unwrap();
        let price_table: HashMap<String, f32> = price_list
            .into_iter()
            .map(|item| (item.name, item.custom_avg))
            .collect();

        let text =
            read_to_string(filtered_items.unwrap_or_else(|| Path::new("filtered_items.json")))
                .unwrap();
        let filtered_items: FilteredItems = serde_json::from_str(&text).unwrap();

        let items = filtered_items
            .eqmt
            .iter()
            .flat_map(|(_name, equipment_item)| {
                equipment_item
                    .parts
                    .iter()
                    .filter_map(|(name, ducat_item)| {
                        let platinum = *price_table.get(name)?;
                        let ducats = ducat_item.ducats;

                        let item_is_part = name.ends_with("Systems")
                            || name.ends_with("Neuroptics")
                            || name.ends_with("Chassis")
                            || name.ends_with("Harness")
                            || name.ends_with("Wings");
                        let name = match equipment_item.item_type {
                            EquipmentType::Warframes | EquipmentType::Archwing if item_is_part => {
                                name.to_owned() + " Blueprint"
                            }
                            _ => name.to_owned(),
                        };

                        Some(Item {
                            name,
                            platinum,
                            ducats,
                        })
                    })
            })
            .chain(
                filtered_items
                    .ignored_items
                    .iter()
                    .map(|(name, _item)| Item {
                        name: name.to_owned(),
                        platinum: 0.0,
                        ducats: 0,
                    }),
            )
            .collect();

        Database { items }
    }

    pub fn find_item(&self, needle: &str, threshold: Option<usize>) -> Option<&Item> {
        let best_match = self
            .items
            .iter()
            .filter(|item| !item.name.ends_with("Set"))
            .min_by_key(|item| levenshtein(&item.name, needle));

        best_match.and_then(|item| {
            if levenshtein(&item.name.replace(" ", ""), needle)
                <= threshold.unwrap_or(item.name.len() / 3)
            {
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
            .find_item("TitaniaPrimeBlueprint", Some(0))
            .expect("Failed to find Titania Prime Blueprint in database");
        assert_eq!(item.name, "Titania Prime Blueprint");

        let item = db
            .find_item("OctaviaPrimeBlueprint", Some(0))
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
