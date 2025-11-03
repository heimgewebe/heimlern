### 📄 crates/heimlern-bandits/Cargo.toml

**Größe:** 333 B | **md5:** `b1d2cf5a726f358a29e52f27d7c61972`

```toml
[package]
name = "heimlern-bandits"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Simple bandit policies for heimlern"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rand = "0.8"
heimlern-core = { path = "../heimlern-core" }
thiserror = "1"

[dev-dependencies]
serde_json = "1"
```

