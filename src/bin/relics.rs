use wfinfo::{database::Database, wfinfo_data::item_data::Refinement};

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
        s => panic!("Invalid relic type: {s}"),
    };

    let relic_count: u32 = args
        .next()
        .unwrap_or("4".to_string())
        .parse()
        .expect("Failed to parse relic count");

    let mut sorted_relics: Vec<(String, f32)> = relics
        .iter()
        .map(|(name, item)| {
            (
                name.to_owned(),
                database.shared_relic_value(&item, Refinement::Radiant, relic_count),
            )
        })
        .collect();
    sorted_relics.sort_by(|a, b| a.1.total_cmp(&b.1));

    let list_length = 40;
    sorted_relics
        .iter()
        .take(list_length / 2)
        .for_each(|(name, value)| println!("{}: {}", name, value));
    if sorted_relics.len() > list_length / 2 {
        println!("...");
        sorted_relics
            .iter()
            .rev()
            .take((list_length / 2).min(sorted_relics.len() - (list_length / 2)))
            .rev()
            .for_each(|(name, value)| println!("{}: {}", name, value));
    }
}
