use std::fs::read_to_string;

use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    name: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    yesterday_vol: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    today_vol: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    custom_avg: f32,
}

pub struct Database {
    items: Vec<Item>,
}

pub fn load_database() -> Database {
    // download file from: https://api.warframestat.us/wfinfo/prices
    let text = read_to_string("prices.json").unwrap();
    let items: Vec<Item> = serde_json::from_str(&text).unwrap();

    Database { items }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn can_load_database() {
        load_database();
    }
}
