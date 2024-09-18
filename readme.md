# treee

Point cloud importer and viewer for forest scans.

## Run Program

### Webdemo

- Availabe at <https://www.wetzel.xyz/treee/index.html>
- Limited to a maximum of 15 million points

### Download

- Availabe at <https://github.com/antonWetzel/treee/releases> for windows
- Download and start `treee.exe`

### Compile

- Requires Rust toolchain
- Execute `cargo install --git=https://github.com/antonWetzel/treee.git --locked`
- Run `treee`

## Usage

1. Load source file
    - `.las` and `.laz` files are supported
2. Automatic calculation of segments for every tree
    - minimal distance between between segments can be changed
3. Automatic calculation of characteristics for every segment
4. Interactive view
    - remove points
    - create new segment
    - change segment for points
    - select tree for focused view
5. Focused tree view
    - remove points
    - change trunk starting height
    - change crown starting height
    - calculate convex hull for the crown

## Camera Controls

- <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd>: Move horizontal
- <kbd>Q</kbd> <kbd>E</kbd>: Move vertical 
- Left Mouse Button: Select or Pan
- Right Mouse Button: Edit
