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
		- content will be deleted
	- wait for import phase
		- viewer can open the project after the import phase
	- wait for calculate phase

## Viewer

- `cargo run --bin=viewer (--release)`
	- select project file
		- `project.epc` in output folder
	- controls (work in progress)
		- <kbd>w</kbd><kbd>a</kbd><kbd>s</kbd><kbd>d</kbd> move camera
		- <kbd>left mouse</kbd> pan camera
		- <kbd>c</kbd> change camera controller
			- panning controls (default)
			- first person
		- <kbd>l</kbd> change level of detail algorithm
			- based on distance (default)
			- equal level for all
		- level of detail quality
			- <kbd>r</kbd> increase
			- <kbd>f</kbd> decrease
		- eye dome lightning
			- <kbd>u</kbd>/<kbd>i</kbd> increase/decrease strength
			- <kbd>j</kbd>/<kbd>k</kbd> increase/decrease sensitivity
		- buttons
			- `Folder`: open other project file
			- `Bug`: open/close debug information
			- `Color Palette`: change visualization color palette

## Notes

### To-do

- only use import folders if the folder is empty or a `project.epc` is present
- ...
