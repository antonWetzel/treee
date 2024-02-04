# treee

Point cloud importer and viewer with focus on forest scans.

## Usage

### Run Project

- Download project `git clone https://github.com/antonWetzel/treee.git`
- Install Rust <https://www.rust-lang.org/tools/install>
- Install and Run 
	- > cargo install --path=treee --locked
	- > treee
- or run directly by replacing `treee <...>` with
	- > cargo run [--release] -- <...>

### Help

- > treee
- > treee help importer
- > treee help viewer

## Importer

- `treee importer`
	- see `treee help importer` for options
	- select input file (`.las` or `.laz`)
	- select empty output folder
	- phases
		1. setup files
		1. import points
		1. segment into trees
		1. calculate information about segments
		1. create project file
		1. save data and level of detail

## Viewer

- `treee viewer`
	- select project file
		- `project.epc` in output folder
	- UI for settings
	- navigate with <kbd>wasd</kbd> or <kbd>↑ ← ↓ →</kbd>
	- left mouse button to pan the camera
	- click to select segment
