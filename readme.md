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
		- `Close`: Close application
		- `Folder`: open project file
		- `Bug`: toggle debug information
			- hidden (default)
			- visible
		- `Color Palette`: toggle visualization color palette
			- warm (default)
			- cold
		- `Information`: change visualization property
			- height (default)
			- inverse height
		- `Invert`: toggle eye dome lightning
			- active (default)
			- inactive
		- `Invert Popup Buttons`
			- increase/decrease strength
			- increase/decrease sensitivity
		- `Camera`: toggle camera controller
			- orbital controls (default)
			- first person
		- `Level of Detail`: change level of detail calculation
			- based on distance (default)
			- equal level for all
		- `Level of Detail Popup Button`
			- increase/decrease quality
		- `Slice`
			- hover for sliders
		- `Slice Popup Sliders`
			- change min and max value for property
			- points outside the range are hidden
		- `Box`
			- reset selected Segment
