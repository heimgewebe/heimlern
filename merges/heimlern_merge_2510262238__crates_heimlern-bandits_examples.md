### 📄 crates/heimlern-bandits/examples/decide.rs

**Größe:** 1 KB | **md5:** `7028e6311661a4f6b3b52b1efa1ea8f3`

```rust
use std::io::{self, Read};

use heimlern_bandits::RemindBandit;
use heimlern_core::{Context, Policy};
use serde_json::{json, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let ctx = if input.trim().is_empty() {
        Context {
            kind: "reminder".into(),
            features: json!({}),
        }
    } else {
        match serde_json::from_str::<Context>(&input) {
            Ok(ctx) => ctx,
            Err(_) => match serde_json::from_str::<Value>(&input) {
                Ok(Value::Object(mut obj)) => {
                    let kind = obj
                        .remove("kind")
                        .and_then(|v| v.as_str().map(|s| s.to_owned()))
                        .unwrap_or_else(|| "reminder".to_string());
                    let features = obj.remove("features").unwrap_or_else(|| json!({}));
                    Context { kind, features }
                }
                Ok(Value::String(kind)) => Context {
                    kind,
                    features: json!({}),
                },
                _ => Context {
                    kind: input.trim().into(),
                    features: json!({}),
                },
            },
        }
    };

    let mut policy = RemindBandit::default();
    let decision = policy.decide(&ctx);

    serde_json::to_writer_pretty(io::stdout(), &decision)?;
    println!();

    Ok(())
}
```

### 📄 crates/heimlern-bandits/examples/integrate_hauski.rs

**Größe:** 337 B | **md5:** `24ccc249ffd8fb34a4da34e2f446b510`

```rust
use heimlern_bandits::RemindBandit;
use heimlern_core::{Context, Policy};

fn main() {
    let mut p = RemindBandit::default();
    let ctx = Context {
        kind: "reminder".into(),
        features: serde_json::json!({"load": 0.3}),
    };
    let d = p.decide(&ctx);
    println!("{}", serde_json::to_string_pretty(&d).unwrap());
}
```

