$env:RUSTFLAGS = '-C target-feature=+atomics,+bulk-memory,+mutable-globals'
rustup run nightly wasm-pack build treee-wasm --target web -- -Z build-std=panic_abort,std