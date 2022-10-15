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
                    database.shared_relic_value(&item, refinement, relic_count),
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
