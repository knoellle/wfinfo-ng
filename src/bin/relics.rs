use std::collections::HashMap;

use wfinfo::{
    database::Database,
    wfinfo_data::item_data::{Refinement, Relic},
};

fn relic_values(database: &Database, relics: &HashMap<String, Relic>, relic_count: u32) {
    let mut sorted_relics: Vec<(String, Refinement, f32)> = relics
        .iter()
        .map(|(name, item)| {
            let (refinement, value) = [
                Refinement::Intact,
                Refinement::Exceptional,
                Refinement::Flawless,
                Refinement::Radiant,
            ]
            .into_iter()
            .map(|refinement| {
                (
                    refinement,
                    database.shared_relic_value(item, refinement, relic_count),
                )
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
            (name.to_owned(), refinement, value)
        })
        .collect();
    sorted_relics.sort_by(|a, b| a.2.total_cmp(&b.2));

    let list_length = 40;
    sorted_relics
        .iter()
        .take(list_length / 2)
        .for_each(|(name, refinement, value)| println!("{}:\t{:?}\t{}", name, refinement, value));
    if sorted_relics.len() > list_length / 2 {
        println!("...");
        sorted_relics
            .iter()
            .rev()
            .take((list_length / 2).min(sorted_relics.len() - (list_length / 2)))
            .rev()
            .for_each(|(name, refinement, value)| {
                println!("{}:\t{:?}\t{}", name, refinement, value)
            });
    }
}

fn best_trace_dump(database: &Database) {
    let mut relics = Vec::new();
    for (prefix, relic_group) in [
        ("Lith", &database.relics.lith),
        ("Meso", &database.relics.meso),
        ("Neo", &database.relics.neo),
        ("Axi", &database.relics.axi),
    ] {
        for (name, relic) in relic_group.iter() {
            let intact = database.shared_relic_value(relic, Refinement::Intact, 4);
            let radiant = database.shared_relic_value(relic, Refinement::Radiant, 4);
            relics.push((format!("{prefix} {name}"), radiant - intact));
        }
    }

    let mut sorted_relics = relics;
    sorted_relics.sort_by(|a, b| a.1.total_cmp(&b.1));

    let list_length = 40;
    println!("...");
    sorted_relics
        .iter()
        .rev()
        .take(list_length)
        .rev()
        .for_each(|(name, value)| println!("{}:  \t{}", name, value));
}

fn main() {
    let database = Database::load_from_file(None, None);
    let mut args = std::env::args().skip(1);
    let relics = match args
        .next()
        .expect("No relic type provided")
        .to_lowercase()
        .as_str()
    {
        "lith" => &database.relics.lith,
        "meso" => &database.relics.meso,
        "neo" => &database.relics.neo,
        "axi" => &database.relics.axi,
        "tracedump" => {
            best_trace_dump(&database);
            return;
        }
        s => panic!("Invalid relic type: {s}"),
    };
    let relic_count: u32 = args
        .next()
        .unwrap_or_else(|| "4".to_string())
        .parse()
        .expect("Failed to parse relic count");
    relic_values(&database, relics, relic_count);
}
