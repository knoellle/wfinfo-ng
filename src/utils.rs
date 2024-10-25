use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub fn fetch_prices_and_items() -> Result<(PathBuf, PathBuf), anyhow::Error> {
    let prices = download_and_save("https://api.warframestat.us/wfinfo/prices/", "prices.json")?;
    let items = download_and_save(
        "https://api.warframestat.us/wfinfo/filtered_items/",
        "filtered_items.json",
    )?;
    Ok((prices, items))
}

fn download_and_save(url: &str, filename: &str) -> Result<PathBuf, anyhow::Error> {
    let path = std::env::temp_dir().join(filename);
    if path.exists() {
        return Ok(path);
    }

    let res = reqwest::blocking::get(url)?;
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)?;
    file.write_all(res.text()?.as_bytes())?;

    Ok(path)
}
