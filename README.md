# WFinfo-ng

# Prerequisites

- `rust` rustc >= 1.64 & cargo. I recommend installation via [rustup](https://rustup.rs).
- `libxrandr` for taking screenshots
- `tesseract` for OCR processing
- `curl`, `jq` for updating the databases

# Usage

Run the `update.sh` script to download the latest database files.

Find where your game puts it's `EE.log` file. Mine is located at `.local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log`.

Now run `cargo run --release --bin wfinfo <path to your EE.log file>`
This will compile (if necessary) and run the program, immediately taking a screenshot and analyzing it, see section Issues and Workarounds for why this is necessary.
Then it continuously scans the log file to look for the reward screen event, trying to detect items in the screenshot.

Once items are found, their platinum and ducat values are looked up in the database downloaded previously.
Each item is printed to stdout along with it's platinum and ducat value in platinum (assuming 10:1 conversion).
The highest value item is also indicated with a little arrow.
When the highest value is determined by the ducat value and there is more than one item with the same ducat value, the platinum values are used as a tie breaker.

# Issue and Workarounds

- Due to buffering when the game writes the `EE.log` file, it is possible that WFInfo doesn't pick up the reward screen event until the screen has disappeared. I haven't found a way of getting around the buffered writer.
  To work around this, you can simply restart the program when it doesn't pick up the reward screen within a few seconds.
- The game data currently contains a couple relics that don't have any items in them, yet are listed in the database.
  This results in error messages like `missing field 'rare1'`. To fix it, simply remove all relics that only contain a `vaulted` key but not any items from `filtered_items.json`.
