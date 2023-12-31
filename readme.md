# treee

Point cloud importer and viewer with focus on forest scans.

## Usage

### Prerequisites

- Download project `git clone https://github.com/antonWetzel/treee.git`
- Install Rust <https://www.rust-lang.org/tools/install>

## Importer

- `cargo run --bin=importer (--release)`
	- select input file (`.las` or `.laz`)
	- select empty output folder
	- phases
		1. open input file
		1. import points
		1. segment into trees
		1. calculate data from the points
		1. crate project file
		1. save data and level of detail

## Viewer

- `cargo run --bin=viewer (--release)`
	- select project file
		- `project.epc` in output folder
	- UI for settings
	- navigate with <kbd>wasd</kbd> or <kbd>↑←↓→</kbd>
	- left mouse button to pan the camera
	- click to select segment
