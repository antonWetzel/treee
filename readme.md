# treee

## todo

- importer
	- only save position in import phase
	- clear up when to flatten
	- use NonZeroUSize for node indices for Option optimization
- render
	- separate static from dynamic things to reduce duplication in X::new(...) and x.update(...)
	- color as 1d texture lookup with custom switchable property array
- viewer
	- fix crash on minimize
	- render parameters
		- eye dome sensitivity and strength
		- color by height
	- load priority
