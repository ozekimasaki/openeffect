# expr

After Effects の JavaScript 式を評価する。

rusty_v8 0.32.1（この環境の crates.io で取れる版）は依存 home が edition 2024 を要求し、Cargo 1.83 ではビルドできないため、同じ公開 API の純 Rust 評価器で動かしている。
