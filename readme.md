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
	- controls (work in progress)
		- <kbd>wasd</kbd>/<kbd>↑←↓→</kbd>move camera
		- <kbd>left mouse button</kbd> pan camera
		- `Folder`: open project file
		- `Bucket`: background color
			- click for reset
			- popup sliders for fine control
		- `Color Palette`: toggle visualization color palette
			- warm (default)
			- cold
			- green/brown
		- `Information`: change visualization property
			- height (default)
			- inverse height
		- `Invert`: eye dome lightning
			- click to toggle
			- popup slider for strength or color
		- `Camera`: toggle camera controller
			- orbital controls (default)
			- first person
		- `Layers`: level of detail
			- click to change
				- based on distance (default)
				- equal level for all
			- popup buttons to increase or decrease quality
		- `Box`: segment reset
			- click to reset selected Segment
		- `Sliders`: view slice
			- popup sliders to set min and max value
