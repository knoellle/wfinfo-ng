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
        Warframes,
        Primary,
        Secondary,
        Melee,
        Sentinels,
        Archwing,
        #[serde(rename = "Arch-Gun")]
        ArchGun,
        Skins,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct EquipmentItem {
        #[serde(rename = "type")]
        pub item_type: EquipmentType,
        pub vaulted: bool,
        pub parts: HashMap<String, DucatItem>,
    }

    #[derive(Copy, Clone, Debug)]
    pub enum Refinement {
        Intact,
        Exceptional,
        Flawless,
        Radiant,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Relic {
        pub vaulted: bool,
        pub rare1: String,
        pub uncommon1: String,
        pub uncommon2: String,
        pub common1: String,
        pub common2: String,
        pub common3: String,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Relics {
        #[serde(rename = "Lith")]
        pub lith: HashMap<String, Relic>,
        #[serde(rename = "Neo")]
        pub neo: HashMap<String, Relic>,
        #[serde(rename = "Meso")]
        pub meso: HashMap<String, Relic>,
        #[serde(rename = "Axi")]
        pub axi: HashMap<String, Relic>,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct FilteredItems {
        pub errors: Vec<String>,
        pub relics: Relics,
        pub eqmt: HashMap<String, EquipmentItem>,
        pub ignored_items: HashMap<String, DucatItem>,
    }

    impl Refinement {
        pub fn common_chance(&self) -> f32 {
            match self {
                Refinement::Intact => 0.2533,
                Refinement::Exceptional => 0.2333,
                Refinement::Flawless => 0.2,
                Refinement::Radiant => 0.1667,
            }
        }

        pub fn uncommon_chance(&self) -> f32 {
            match self {
                Refinement::Intact => 0.11,
                Refinement::Exceptional => 0.13,
                Refinement::Flawless => 0.17,
                Refinement::Radiant => 0.20,
            }
        }

        pub fn rare_chance(&self) -> f32 {
            match self {
                Refinement::Intact => 0.02,
                Refinement::Exceptional => 0.04,
                Refinement::Flawless => 0.06,
                Refinement::Radiant => 0.1,
            }
        }
    }
}
