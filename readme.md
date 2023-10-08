# treee

## todo

- importer
	- only save position in import phase
	- clear up when to flatten
	- use NonZeroUSize for node indices for Option optimization
- render
	- arguments for required state with `Has<X>`
	- seperate static from dynamic things to reduce duplication in X::new(...) and x.update(...)
	- eye dome: mark in pixel space
- viewer
	- fix crash on minimize
	- render parameters
		- eye dome sensitivity and strengh
		- color by height
	- load priority
