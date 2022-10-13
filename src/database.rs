use std::{collections::HashMap, fs::read_to_string, path::Path};

use levenshtein::levenshtein;
use serde::Deserialize;

use crate::{
    statistics::{self, Bucket},
    wfinfo_data::{
        item_data::{EquipmentType, FilteredItems, Refinement, Relic, Relics},
        price_data::PriceItem,
    },
};

#[derive(Clone, Debug, Deserialize)]
pub struct Database {
    items: Vec<Item>,
    pub relics: Relics,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    pub name: String,
    pub drop_name: String,
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
                        let item_is_part = name.ends_with("Systems")
                            || name.ends_with("Neuroptics")
                            || name.ends_with("Chassis")
                            || name.ends_with("Harness")
                            || name.ends_with("Wings");
                        let drop_name = match equipment_item.item_type {
                            EquipmentType::Warframes | EquipmentType::Archwing if item_is_part => {
                                name.to_owned() + " Blueprint"
                            }
                            _ => name.to_owned(),
                        };
                        let platinum = *match price_table
                            .get(name)
                            .or_else(|| price_table.get(&format!("{name} Blueprint")))
                        {
                            Some(plat) => plat,
                            None => {
                                println!("Failed to find price for item: {name}");
                                return None;
                            }
                        };
                        let ducats = ducat_item.ducats;

                        Some(Item {
                            name: name.to_string(),
                            drop_name,
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
                        drop_name: name.to_owned(),
                        platinum: 0.0,
                        ducats: 0,
                    }),
            )
            .collect();
        let relics = filtered_items.relics;

        Database { items, relics }
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

    pub fn find_item_exact(&self, needle: &str) -> Option<&Item> {
        self.items.iter().find(|item| item.name == needle)
    }

    fn relic_to_bucket(&self, relic: &Relic, refinement: Refinement) -> Bucket {
        let common_chance = refinement.common_chance();
        let uncommon_chance = refinement.uncommon_chance();
        let rare_chance = refinement.rare_chance();

        let item_names = [
            (&relic.common1, common_chance),
            (&relic.common2, common_chance),
            (&relic.common3, common_chance),
            (&relic.uncommon1, uncommon_chance),
            (&relic.uncommon2, uncommon_chance),
            (&relic.rare1, rare_chance),
        ];
        let items = item_names
            .into_iter()
            .map(|(name, chance)| statistics::Item {
                value: self
                    .find_item_exact(name)
                    .expect(&format!("Failed to find item {} in database", name))
                    .platinum,
                probability: chance,
            })
            .collect();
        Bucket::new(items)
    }

    pub fn single_relic_value(&self, relic: &Relic, refinement: Refinement) -> f32 {
        let common_chance = refinement.common_chance();
        let uncommon_chance = refinement.uncommon_chance();
        let rare_chance = refinement.rare_chance();

        let value = 0.0
            + self.find_item_exact(&relic.common1).unwrap().platinum * common_chance
            + self.find_item_exact(&relic.common2).unwrap().platinum * common_chance
            + self.find_item_exact(&relic.common3).unwrap().platinum * common_chance
            + self.find_item_exact(&relic.uncommon1).unwrap().platinum * uncommon_chance
            + self.find_item_exact(&relic.uncommon2).unwrap().platinum * uncommon_chance
            + self.find_item_exact(&relic.rare1).unwrap().platinum * rare_chance;

        let item_names = [
            (&relic.common1, common_chance),
            (&relic.common2, common_chance),
            (&relic.common3, common_chance),
            (&relic.uncommon1, uncommon_chance),
            (&relic.uncommon2, uncommon_chance),
            (&relic.rare1, rare_chance),
        ];
        let value2: f32 = item_names
            .into_iter()
            .map(|(name, chance)| {
                let plat = self.find_item_exact(name).unwrap().platinum;
                println!("{plat} * {chance}");
                plat * chance
            })
            .sum();
        println!("{value} vs {value2}");

        return value;
    }

    pub fn shared_relic_value(
        &self,
        relic: &Relic,
        refinement: Refinement,
        number_of_relics: i32,
    ) -> f32 {
        let bucket = self.relic_to_bucket(relic, refinement);
        bucket.expectation_of_best_of_n(number_of_relics)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn can_load_database() {
        Database::load_from_file(None, None);
    }

    #[test]
    pub fn can_find_items() {
        let db = Database::load_from_file(None, None);

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
        let db = Database::load_from_file(None, None);

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
