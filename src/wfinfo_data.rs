use serde::Deserialize;
use serde_aux::prelude::deserialize_number_from_string;

pub mod price_data {
    use super::*;

    #[derive(Clone, Debug, Deserialize)]
    pub struct PriceItem {
        pub name: String,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        pub custom_avg: f32,
    }
}

pub mod item_data {
    use std::collections::HashMap;

    use super::*;

    #[derive(Clone, Debug, Deserialize)]
    pub struct DucatItem {
        #[serde(default)]
        pub ducats: usize,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub enum EquipmentType {
        Primary,
        Secondary,
        Warframe,
        Sentinel,
        Archwing,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct EquipmentItem {
        #[serde(rename = "type")]
        pub item_type: String,
        pub vaulted: bool,
        pub parts: HashMap<String, DucatItem>,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct FilteredItems {
        pub errors: Vec<String>,
        pub eqmt: HashMap<String, EquipmentItem>,
        pub ignored_items: HashMap<String, DucatItem>,
    }
}
