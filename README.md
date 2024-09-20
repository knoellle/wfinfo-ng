# WFinfo-ng

A Linux compatible version of the great [WFinfo](https://github.com/WFCD/WFinfo/).

Does support:

- Detecting relic reward screens
- Taking a screenshot the game
- Detecting items
- Displaying platinum values for each item
- X11 & Wayland
- Game in windowed or fullscreen mode

Doesn't support:

- Market integration
- Inventory tracking
- Interactive "snap-it" features

# Prerequisites and Dependencies

- `rust` rustc >= 1.74 & cargo. I recommend installation via [rustup](https://rustup.rs).
- `libxrandr` for taking screenshots
- `tesseract` for OCR processing
- `curl`, `jq` for updating the databases

# Installation

1. Clone this repository
1. Install only reward screen helper: `cargo install --path . --bin wfinfo`
1. Or install all tools: `cargo install --path .`

# Usage

Run the `update.sh` script to download the latest database files.

Find where your game puts it's `EE.log` file. Mine is located at `.local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log`.

Now run `wfinfo <path to your EE.log file>`
This will run the program, immediately taking a screenshot and analyzing it, see section Issues and Workarounds for why.
The program then waits for the reward screen, trying to detect items in the screenshot.

Once items are found, their platinum and ducat values are looked up in the database downloaded previously.
Each item is printed to stdout along with it's platinum and ducat value in platinum (assuming 10:1 conversion).
The highest value item is also indicated with a little arrow.
When the highest value is determined by the ducat value and there is more than one item with the same ducat value, the platinum values are used as a tie breaker.

# Issue and Workarounds

- Due to buffering when the game writes the `EE.log` file, it is possible that WFInfo doesn't pick up the reward screen event until the screen has disappeared. I haven't found a way of getting around the buffered writer.
  If this happens, you can manually trigger the detection by pressing the F12 key.

- If you are using gamescope add the flag `--window-name=gamescope`
