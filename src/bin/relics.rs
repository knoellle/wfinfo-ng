use wfinfo::{database::Database, wfinfo_data::item_data::Refinement};

fn main() {
    let database = Database::load_from_file(None, None);
    let (name, relic) = database.relics.neo.iter().next().unwrap();
    println!("Neo {} {:#?}", name, relic);
    println!(
        "Single: {} {}",
        database.single_relic_value(relic, Refinement::Radiant),
        database.shared_relic_value(relic, Refinement::Radiant, 1),
    );
    println!(
        "Four: {}",
        database.shared_relic_value(relic, Refinement::Radiant, 4),
    );
}
