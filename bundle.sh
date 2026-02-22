rustup target add wasm32-unknown-unknown

cargo install dioxus-cli

dx bundle --release --platform web
