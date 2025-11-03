### 📄 merges/heimlern_merge_2510262237__.github_workflows.md

**Größe:** 5 KB | **md5:** `e5519251df509e579ac9b973d0ed9979`

```markdown
### 📄 .github/workflows/ci-rust.yml

**Größe:** 1 KB | **md5:** `0557d72bf7beeb0ad27e7edc112a8eb5`

```yaml
name: rust (cached)

on:
  push:
    paths:
      - "Cargo.toml"
      - "Cargo.lock"
      - "crates/**"
      - ".github/workflows/ci-rust.yml"
  pull_request:
    paths:
      - "Cargo.toml"
      - "Cargo.lock"
      - "crates/**"
      - ".github/workflows/ci-rust.yml"

permissions:
  contents: read

jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust (stable)
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache target
        uses: actions/cache@v4
        with:
          path: target
          key: ${{ runner.os }}-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy lint
        run: cargo clippy --workspace --all-targets --locked -- -D warnings

      - name: Build workspace
        run: cargo build --workspace --all-targets --locked

      - name: Test workspace
        run: cargo test --workspace --all-targets --locked --no-fail-fast
```

### 📄 .github/workflows/ci.yml

**Größe:** 455 B | **md5:** `789b20eee1f28d4998da06fd5df06b31`

```yaml
name: ci
on: [push, pull_request]
permissions:
  contents: read
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --workspace --all-targets --verbose
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: echo '{}' | cargo run -p heimlern-bandits --example decide
      - run: cargo test --workspace --all-targets --verbose
```

### 📄 .github/workflows/contracts.yml

**Größe:** 706 B | **md5:** `b2be86b2a1e0a2a3b057acca093fc5b5`

```yaml
name: contracts
permissions:
  contents: read
on:
  push:
  pull_request:
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.x'
      - run: python -m pip install --upgrade pip
      - run: pip install -r requirements-tools.txt
      - name: Generate examples
        run: python scripts/examples.py
      - name: Validate snapshot
        run: python scripts/validate_json.py contracts/policy_snapshot.schema.json /tmp/heimlern_snapshot.json
      - name: Validate feedback
        run: python scripts/validate_json.py contracts/policy_feedback.schema.json /tmp/heimlern_feedback.json
```

### 📄 .github/workflows/validate-aussen-samples.yml

**Größe:** 526 B | **md5:** `71d9760b4539abba75abb7a772ec3649`

```yaml
name: validate (aussen samples)
on: [push, pull_request, workflow_dispatch]

permissions:
  contents: read

jobs:
  validate:
    if: ${{ hashFiles('data/samples/aussensensor.jsonl') != '' }}
    uses: heimgewebe/metarepo/.github/workflows/reusable-validate-jsonl.yml@contracts-v1
    with:
      jsonl_paths_list: |
        data/samples/aussensensor.jsonl
      schema_url: https://raw.githubusercontent.com/heimgewebe/metarepo/contracts-v1/contracts/aussen.event.schema.json
      strict: false
      validate_formats: true
```

### 📄 .github/workflows/validate-aussen.yml

**Größe:** 1001 B | **md5:** `7180dd9eaa083d00a579f0d78af1fca1`

```yaml
name: validate (aussen in heimlern)
permissions:
  contents: read
on: [push, pull_request, workflow_dispatch]
jobs:
  samples:
    name: samples (data/samples/aussensensor.jsonl)
    if: ${{ hashFiles('data/samples/aussensensor.jsonl') != '' }}
    uses: heimgewebe/metarepo/.github/workflows/reusable-validate-jsonl.yml@contracts-v1
    with:
      jsonl_path: data/samples/aussensensor.jsonl
      schema_url: https://raw.githubusercontent.com/heimgewebe/metarepo/contracts-v1/contracts/aussen.event.schema.json
      strict: false
      validate_formats: true

  fixtures:
    name: fixtures (tests/fixtures/aussen.jsonl)
    if: ${{ hashFiles('tests/fixtures/aussen.jsonl') != '' }}
    uses: heimgewebe/metarepo/.github/workflows/reusable-validate-jsonl.yml@contracts-v1
    with:
      jsonl_path: tests/fixtures/aussen.jsonl
      schema_url: https://raw.githubusercontent.com/heimgewebe/metarepo/contracts-v1/contracts/aussen.event.schema.json
      strict: false
      validate_formats: true
```
```

### 📄 merges/heimlern_merge_2510262237__contracts.md

**Größe:** 4 KB | **md5:** `d458281125df1ea836027397d17ef05b`

```markdown
### 📄 contracts/README.md

**Größe:** 576 B | **md5:** `bc7f4000f172ba170998c78dcd6a8442`

```markdown
# Contracts für heimlern (Snapshots & Feedback)

Diese Verträge definieren das externe Austauschformat:
- **PolicySnapshot**: Zustandsstand einer Policy (Arme, Zähler, Werte …)
- **PolicyFeedback**: Rückmeldung zu einer Entscheidung (Reward, Notizen)
- **Außensensor-Event**: Normalisierte JSON-Struktur für eingehende Sensor-Events

Ziele:
- Reproduzierbarkeit (Versionierung)
- Strikte Validierung (keine schleichende Schema-Drift)
- Tool-agnostisch (Rust, Python, Shell …)

## Quickstart
```sh
just snapshot:example
just feedback:example
just schema:validate
```
```

### 📄 contracts/aussen_event.schema.json

**Größe:** 2 KB | **md5:** `a921ba479db5173dd1af312d39b1c320`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://heimlern.schemas/aussen-event.schema.json",
  "title": "Außensensor-Event",
  "description": "Normiertes Ereignis aus externen Sensoren oder Integrationen.",
  "type": "object",
  "required": ["type", "source"],
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string",
      "description": "Eindeutige Kennung des Events (frei wählbar)."
    },
    "type": {
      "type": "string",
      "description": "Kategorie des Events, z. B. 'link' oder 'note'."
    },
    "source": {
      "type": "string",
      "description": "Herkunft oder Integrationsquelle, z. B. 'sensor-hof'."
    },
    "title": {
      "type": "string",
      "description": "Optionaler Titel zur Anzeige."
    },
    "summary": {
      "type": "string",
      "description": "Kurzbeschreibung oder Erklärung zum Event."
    },
    "url": {
      "type": "string",
      "format": "uri",
      "description": "Referenz-URL für weiterführende Informationen."
    },
    "tags": {
      "type": "array",
      "description": "Kategorisierende Tags.",
      "items": {
        "type": "string"
      }
    },
    "ts": {
      "type": "string",
      "format": "date-time",
      "description": "Zeitstempel im ISO-8601-Format."
    },
    "features": {
      "type": "object",
      "description": "Beliebige strukturierte Zusatzinformationen.",
      "additionalProperties": true
    },
    "meta": {
      "type": "object",
      "description": "Weitere Metadaten ohne feste Struktur.",
      "additionalProperties": true
    }
  },
  "examples": [
    {
      "type": "link",
      "source": "sensor-hof",
      "title": "Regenwarnung",
      "url": "https://example.com/wetter",
      "tags": ["regen", "sensor"],
      "features": {
        "level": 0.82
      }
    }
  ]
}
```

### 📄 contracts/policy_feedback.schema.json

**Größe:** 579 B | **md5:** `b1178697a59d0a7b73d9f7d3c4d4fe0a`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://heimgewebe/contracts/policy_feedback.schema.json",
  "title": "PolicyFeedback",
  "type": "object",
  "additionalProperties": false,
  "required": ["version","policy_id","ts","decision_id","reward"],
  "properties": {
    "version": { "type": "string" },
    "policy_id": { "type": "string", "minLength": 1 },
    "ts": { "type": "string", "format": "date-time" },
    "decision_id": { "type": "string", "minLength": 1 },
    "reward": { "type": "number" },
    "notes": { "type": "string" }
  }
}
```

### 📄 contracts/policy_snapshot.schema.json

**Größe:** 859 B | **md5:** `40baa78d6951bcdbe5f2fa9fdc495a07`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://heimgewebe/contracts/policy_snapshot.schema.json",
  "title": "PolicySnapshot",
  "type": "object",
  "additionalProperties": false,
  "required": ["version","policy_id","ts","arms","counts","values","epsilon"],
  "properties": {
    "version": { "type": "string" },
    "policy_id": { "type": "string", "minLength": 1 },
    "ts": { "type": "string", "format": "date-time" },
    "arms": {
      "type": "array",
      "minItems": 1,
      "items": { "type": "string", "minLength": 1 }
    },
    "counts": {
      "type": "array",
      "items": { "type": "integer", "minimum": 0 }
    },
    "values": {
      "type": "array",
      "items": { "type": "number" }
    },
    "epsilon": { "type": "number", "minimum": 0, "maximum": 1 },
    "seed": { "type": "integer" }
  }
}
```
```

### 📄 merges/heimlern_merge_2510262237__crates_heimlern-bandits.md

**Größe:** 457 B | **md5:** `cda2976f07282ddcacbde93f7ecd7ab5`

```markdown
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
```

### 📄 merges/heimlern_merge_2510262237__crates_heimlern-bandits_examples.md

**Größe:** 2 KB | **md5:** `822ae7d4b7da15ef2c6f6d4fbfd364c5`

```markdown
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
```

### 📄 merges/heimlern_merge_2510262237__crates_heimlern-bandits_src.md

**Größe:** 8 KB | **md5:** `cc61ac2351ba2284aca6598486247c89`

```markdown
### 📄 crates/heimlern-bandits/src/error.rs

**Größe:** 352 B | **md5:** `4dd44e6ba79010a5a56c348149459ed3`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BanditError {
    #[error("Snapshot deserialization failed: {0}")]
    Snapshot(#[from] serde_json::Error),
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    #[error("Internal error: {0}")]
    Internal(&'static str),
}

pub type Result<T> = std::result::Result<T, BanditError>;
```

### 📄 crates/heimlern-bandits/src/lib.rs

**Größe:** 7 KB | **md5:** `b47d33949337cf6f4294f7da4928a8bd`

```rust
#![warn(clippy::unwrap_used, clippy::expect_used)]

//! Beispiel-Implementierung eines ε-greedy-Banditen für Erinnerungs-Slots.
//!
//! Der `RemindBandit` implementiert das [`Policy`](heimlern_core::Policy)-Trait
//! für ein häusliches Erinnerungs-Szenario. Mit Wahrscheinlichkeit `epsilon` wird
//! ein Slot zufällig gewählt (Exploration), sonst der beste bekannte Slot (Exploitation).

// Fehler-Typ für zukünftige Refactors (unwrap() -> Result)
pub mod error;
pub use error::{BanditError, Result};

use heimlern_core::{Context, Decision, Policy};
use rand::prelude::*;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_SLOTS: &[&str] = &["morning", "afternoon", "evening"];

/// ε-greedy Policy für Erinnerungen.
#[derive(Debug, Serialize, Deserialize)]
pub struct RemindBandit {
    /// Wahrscheinlichkeit für Exploration zwischen 0.0 und 1.0.
    pub epsilon: f32,
    /// Verfügbare Zeit-Slots (Arme).
    pub slots: Vec<String>,
    /// Statistiken je Slot: (Anzahl Ziehungen, summierte Rewards).
    values: HashMap<String, (u32, f32)>,
}

impl Default for RemindBandit {
    fn default() -> Self {
        Self {
            epsilon: 0.2,
            slots: default_slots(),
            values: HashMap::new(),
        }
    }
}

fn default_slots() -> Vec<String> {
    DEFAULT_SLOTS.iter().map(|s| (*s).to_string()).collect()
}

fn serialize_context(ctx: &Context) -> Option<serde_json::Value> {
    serde_json::to_value(ctx).ok()
}

fn fallback_decision(reason: &str, ctx: &Context) -> Decision {
    Decision {
        action: "remind.none".into(),
        score: 0.0,
        why: reason.into(),
        context: serialize_context(ctx),
    }
}

impl RemindBandit {
    /// Berechnet den durchschnittlichen Reward für einen Slot.
    fn get_average_reward(&self, slot: &str) -> f32 {
        self.values
            .get(slot)
            .map(|(n, v)| if *n > 0 { v / *n as f32 } else { 0.0 })
            .unwrap_or(0.0)
    }

    fn sanitize(&mut self) {
        if !self.epsilon.is_finite() {
            self.epsilon = 0.0;
        } else {
            self.epsilon = self.epsilon.clamp(0.0, 1.0);
        }

        if self.slots.is_empty() {
            self.slots = default_slots();
        }
    }
}

impl Policy for RemindBandit {
    /// Wählt einen Erinnerungs-Slot basierend auf ε-greedy.
    fn decide(&mut self, ctx: &Context) -> Decision {
        let mut rng = thread_rng();

        self.sanitize();

        // Wenn aus irgendeinem Grund immer noch leer: sichere Rückgabe.
        if self.slots.is_empty() {
            return fallback_decision("no slots available", ctx);
        }

        let explore = rng.gen::<f32>() < self.epsilon;

        let chosen_slot = if explore {
            // Exploration: zufällig wählen (safe, da nicht leer, aber defensiv).
            if let Some(slot) = self.slots.choose(&mut rng) {
                slot.clone()
            } else {
                return fallback_decision("no slots available", ctx);
            }
        } else {
            // Exploitation: Slot mit höchstem durchschnittlichem Reward.
            if let Some(slot) = self.slots.iter().max_by(|a, b| {
                self.get_average_reward(a)
                    .partial_cmp(&self.get_average_reward(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
                slot.clone()
            } else {
                return fallback_decision("no slots available", ctx);
            }
        };

        let value_estimate = self.get_average_reward(&chosen_slot);

        Decision {
            action: format!("remind.{chosen_slot}"),
            score: value_estimate,
            why: if explore { "explore ε" } else { "exploit" }.into(),
            context: serialize_context(ctx),
        }
    }

    /// Nimmt Feedback entgegen und aktualisiert die Schätzung pro Slot.
    fn feedback(&mut self, _ctx: &Context, action: &str, reward: f32) {
        if let Some(slot) = action.strip_prefix("remind.") {
            let entry = self.values.entry(slot.to_string()).or_insert((0, 0.0));
            entry.0 += 1; // pulls
            entry.1 += reward; // total reward
        }
    }

    /// Persistiert vollständigen Zustand als JSON.
    fn snapshot(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Lädt Zustand aus Snapshot (robust mit Korrekturen).
    fn load(&mut self, v: serde_json::Value) {
        if let Ok(mut loaded) = serde_json::from_value::<RemindBandit>(v) {
            loaded.sanitize();
            *self = loaded;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heimlern_core::Policy;

    #[test]
    fn bandit_learns_and_exploits_best_slot() {
        let mut bandit = RemindBandit {
            epsilon: 0.0, // keine Exploration für deterministischen Test
            slots: vec!["morning".into(), "afternoon".into(), "evening".into()],
            values: HashMap::new(),
        };
        let ctx = Context {
            kind: "test".into(),
            features: serde_json::json!({"x":1}),
        };

        // Feedback: "afternoon" ist am besten.
        bandit.feedback(&ctx, "remind.morning", 0.1);
        bandit.feedback(&ctx, "remind.afternoon", 0.9);
        bandit.feedback(&ctx, "remind.evening", 0.3);
        bandit.feedback(&ctx, "remind.afternoon", 0.8);

        let decision = bandit.decide(&ctx);
        assert_eq!(decision.action, "remind.afternoon");
        assert!(decision.score > 0.5);
    }

    #[test]
    fn snapshot_roundtrip_retains_state() {
        let mut bandit = RemindBandit {
            epsilon: 0.33,
            slots: vec!["a".into(), "b".into()],
            values: HashMap::new(),
        };
        let ctx = Context {
            kind: "test".into(),
            features: serde_json::json!({"k":true}),
        };
        bandit.feedback(&ctx, "remind.b", 1.0);

        let snapshot = bandit.snapshot();

        let mut restored = RemindBandit::default();
        restored.load(snapshot);

        assert!((restored.epsilon - 0.33).abs() < f32::EPSILON);
        assert_eq!(restored.slots, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(restored.values.get("b"), Some(&(1, 1.0)));

        restored.epsilon = 0.0;
        let d = restored.decide(&ctx);
        assert_eq!(d.action, "remind.b");
    }

    #[test]
    fn load_clamps_epsilon_and_restores_slots() {
        let bandit = RemindBandit {
            epsilon: 42.0,
            slots: vec![],
            values: HashMap::new(),
        };
        let snapshot = bandit.snapshot();

        let mut restored = RemindBandit::default();
        restored.load(snapshot);

        assert!((restored.epsilon - 1.0).abs() < f32::EPSILON);
        assert_eq!(restored.slots, default_slots());
    }

    #[test]
    fn decisions_have_remind_prefix() {
        let mut bandit = RemindBandit::default();
        let ctx = Context {
            kind: "test".into(),
            features: serde_json::json!({}),
        };

        let decision = bandit.decide(&ctx);
        assert!(decision.action.starts_with("remind."));
    }
}
```
```

### 📄 merges/heimlern_merge_2510262237__crates_heimlern-core.md

**Größe:** 332 B | **md5:** `cc1048887fcee7ac4d7350dac5728223`

```markdown
### 📄 crates/heimlern-core/Cargo.toml

**Größe:** 211 B | **md5:** `deacea9b83c9737c314d1fd2d5808de5`

```toml
[package]
name = "heimlern-core"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Heimlern core traits & types"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```
```

### 📄 merges/heimlern_merge_2510262237__crates_heimlern-core_examples.md

**Größe:** 1 KB | **md5:** `933d90818beea8054262ce0b1b3d6f19`

```markdown
### 📄 crates/heimlern-core/examples/ingest_events.rs

**Größe:** 1 KB | **md5:** `d479419598cd714dfba65b2918710d5a`

```rust
use heimlern_core::event::AussenEvent;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1);
    let reader: Box<dyn BufRead> = match path {
        Some(p) => Box::new(BufReader::new(File::open(p)?)),
        None => Box::new(BufReader::new(io::stdin())),
    };

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let event: AussenEvent = serde_json::from_str(&line)?;

        let mut score: f32 = 0.0;
        if event.url.is_some() {
            score += 0.5;
        }
        if event.title.as_ref().map(|t| !t.is_empty()).unwrap_or(false) {
            score += 0.3;
        }
        if let Some(tags) = &event.tags {
            score += (tags.len().min(5) as f32) * 0.04;
        }

        println!(
            "{score:.2}\t{}",
            event.title.as_deref().unwrap_or("<untitled>")
        );
    }

    Ok(())
}
```
```

### 📄 merges/heimlern_merge_2510262237__crates_heimlern-core_src.md

**Größe:** 5 KB | **md5:** `cd27c4627a4eac698ca96bf4c98fba38`

```markdown
### 📄 crates/heimlern-core/src/event.rs

**Größe:** 2 KB | **md5:** `68188601667f9e2874349084a0a923a2`

```rust
//! Datenstrukturen für externe Events, die von Sensoren oder anderen Quellen
//! stammen.
//!
//! Dieses Modul definiert den [`AussenEvent`], der als standardisiertes
//! Austauschformat für Ereignisse dient, die von außerhalb des Systems
//! eintreffen. Solche Events können beispielsweise von IoT-Geräten, Webhooks
//! oder anderen externen APIs stammen.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Repräsentiert ein externes Ereignis, das von einem Sensor, einer API oder
/// einer anderen Datenquelle stammt.
///
/// Die Struktur ist so konzipiert, dass sie mit dem JSON-Schema in
/// `contracts/aussen_event.schema.json` kompatibel ist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AussenEvent {
    /// Eine eindeutige Kennung für dieses Ereignis, z. B. eine UUID.
    pub id: Option<String>,
    /// Der Typ des Ereignisses, der zur Kategorisierung dient (z. B.
    /// "sensor.reading", "user.interaction"). Entspricht dem `type`-Feld in
    /// JSON.
    #[serde(rename = "type")]
    pub kind: String,
    /// Die Quelle des Ereignisses (z. B. "haus-automation", "user-app").
    pub source: String,
    /// Ein optionaler, menschenlesbarer Titel für das Ereignis.
    pub title: Option<String>,
    /// Eine kurze Zusammenfassung oder Beschreibung des Ereignisses.
    pub summary: Option<String>,
    /// Eine URL, die auf weiterführende Informationen zum Ereignis verweist.
    pub url: Option<String>,
    /// Eine Liste von Tags zur Kategorisierung oder zum Filtern des Ereignisses.
    pub tags: Option<Vec<String>>,
    /// Ein ISO-8601-formatierter Zeitstempel, der angibt, wann das Ereignis
    /// aufgetreten ist.
    pub ts: Option<String>,
    /// Ein flexibles Feld für beliebige strukturierte Daten, die für die
    /// Policy-Entscheidung relevant sind.
    pub features: Option<BTreeMap<String, Value>>,
    /// Zusätzliche Metadaten, die nicht direkt für die Entscheidungsfindung
    /// verwendet werden, aber für Logging oder Debugging nützlich sein können.
    pub meta: Option<BTreeMap<String, Value>>,
}
```

### 📄 crates/heimlern-core/src/lib.rs

**Größe:** 2 KB | **md5:** `c571dd8d28e45abc18da0435821532cb`

```rust
//! Kern-Typen und Traits für das heimlern-Ökosystem.
//!
//! Die hier definierten Strukturen bilden die Schnittstelle zwischen konkreten
//! Policies und der Umgebung, in der Entscheidungen getroffen und bewertet
//! werden. Alle Typen sind `Serialize`/`Deserialize`, damit sie in JSON-basierte
//! APIs, Persistenzschichten oder Tests eingebettet werden können.

pub mod event;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Kontextinformationen, die einer Policy zur Entscheidungsfindung übergeben
/// werden.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Context {
    /// Kategorisierung des Kontextes (z. B. `"reminder"`, `"routine"`).
    pub kind: String,
    /// Beliebige zusätzliche Merkmale als JSON-Struktur.
    pub features: Value,
}

/// Antwort einer Policy auf einen gegebenen [`Context`].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Decision {
    /// Die gewählte Aktion, typischerweise ein identifizierbarer Name oder Slot.
    pub action: String,
    /// Heuristische Bewertung der Aktion. Policies können hier beliebige
    /// numerische Werte verwenden (z. B. gemittelte Rewards ohne Begrenzung).
    pub score: f32,
    /// Erklärung, warum die Aktion gewählt wurde (z. B. "explore ε").
    pub why: String,
    /// Optionaler, serialisierter Kontext (z. B. zum Logging oder Debugging).
    pub context: Option<Value>,
}

/// Schnittstelle, die jede heimlern-Policy implementieren muss.
pub trait Policy {
    /// Wählt eine [`Decision`] für den übergebenen [`Context`].
    fn decide(&mut self, ctx: &Context) -> Decision;

    /// Liefert Rückmeldung über das Ergebnis einer vorherigen Entscheidung.
    fn feedback(&mut self, ctx: &Context, action: &str, reward: f32);

    /// Exportiert den aktuellen internen Zustand als JSON-Snapshot.
    fn snapshot(&self) -> Value;

    /// Lädt einen zuvor erzeugten JSON-Snapshot wieder in die Policy.
    fn load(&mut self, snapshot: Value);
}

// -----------------------
// Tests (Grundabsicherung)
// -----------------------
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn context_roundtrip() {
        let ctx = Context {
            kind: "test".to_string(),
            features: json!({"key": "value", "n": 1}),
        };
        let s = serde_json::to_string(&ctx).unwrap();
        let back: Context = serde_json::from_str(&s).unwrap();
        assert_eq!(ctx.kind, back.kind);
        assert_eq!(ctx.features["key"], "value");
    }
}
```
```

### 📄 merges/heimlern_merge_2510262237__data_samples.md

**Größe:** 562 B | **md5:** `7e4fb7e7ab45800fce1b735b0d7aaf9d`

```markdown
### 📄 data/samples/.gitkeep

**Größe:** 53 B | **md5:** `6225eaccb7b99a6490a0118ac2ed976d`

```plaintext
# Keep data/samples directory under version control.
```

### 📄 data/samples/aussensensor.jsonl

**Größe:** 268 B | **md5:** `96946b86e0dbe4914e33adba90a48c5b`

```plaintext
{"type":"link","source":"sensor-hof","title":"Regenwarnung","url":"https://example.com/wetter","tags":["regen","sensor"],"features":{"level":0.82}}
{"type":"link","source":"sensor-hof","summary":"Keine Daten","url":"https://example.com/offline","meta":{"retry":true}}
```
```

### 📄 merges/heimlern_merge_2510262237__docs.md

**Größe:** 5 KB | **md5:** `d1d945c2c0eb67c3712ab6cde233499d`

```markdown
### 📄 docs/policies-os-context.md

**Größe:** 1 KB | **md5:** `f3ff72422ef8adfeed51eeff785b03f9`

```markdown
# heimlern: Policies für OS-Kontext

## Ziele
- Consent erzwingen
- Sensitive Kontexte blocken
- Rate-Limits & PII-Gate
- Automations (Deep-Work, Selbstheilung)

## Kern-Policies (YAML-Skizze)
```yaml
consent:
  text_capture: false   # muss aktiv vom Nutzer gesetzt werden

pii_gate:
  min_confidence: 0.85
  on_violation: drop_and_shred

rate_limits:
  embed_per_app_per_min: 12
  on_exceed: drop

allow_block:
  allow_apps: [code, obsidian]
  allow_domains: ["localhost", "dev.local"]
  block_apps: ["org.keepassxc.KeePassXC", "com.bank.app"]
  block_domains: ["login.microsoftonline.com", "accounts.google.com"]

modes:
  deep_work:
    enter_if:
      - os.context.state.focus == true
      - hauski_audio.vibe in ["fokussiert", "neutral"]
      - app in ["code", "obsidian"]
    actions:
      - hausKI.hold_notifications
    exit_if:
      - focus == false OR inactivity > 10m
    exit_actions:
      - hausKI.release_notifications
```

## Selbstheilung

- Metriken aus hausKI/wgx beobachten; bei Silence/Latenz → Entscheidung: `wgx doctor`/Restart (lokal), auditierbar.
```

### 📄 docs/policy-lifecycle.md

**Größe:** 3 KB | **md5:** `89e3ddb50dfe689dd1a79174012537f7`

```markdown
# Policy Lifecycle (Snapshot → Validate → Apply)

Dieser Leitfaden beschreibt den Lebenszyklus einer heimlern-Policy von der
Zustandssicherung über die Schema-Validierung bis zur erneuten Aktivierung.
Er ergänzt den Überblick im [README](../README.md) und fokussiert auf
praktische Schritte, die mit den vorhandenen Werkzeugen wiederholt werden
können.

## 1. Snapshot erstellen

1. Policy ausführen und Entscheidung treffen, z. B. über das Beispiel
   `integrate_hauski`:
   ```bash
   cargo run --example integrate_hauski
   ```
2. Der Agent exportiert seinen Zustand als JSON-Snapshot über das
   [`Policy`-Trait](../crates/heimlern-core/src/lib.rs) mit den Methoden
   [`snapshot`](../crates/heimlern-core/src/lib.rs) und
   [`load`](../crates/heimlern-core/src/lib.rs). Dadurch bleiben Zähler,
   Zufalls-Seed und weitere Parameter zwischen Sessions erhalten.
3. Für reproduzierbare Testdaten können die Beispielskripte verwendet werden:
   ```bash
   just snapshot:example
   # legt /tmp/heimlern_snapshot.json an
   ```

Snapshots sind reine JSON-Dokumente, die keine Binärdaten enthalten und in
Versionskontrolle oder Objektspeichern abgelegt werden können.

## 2. Snapshot validieren

1. Werkzeuge vorbereiten (Python-Venv und `jsonschema`):
   ```bash
   just venv
   ```
2. Automatisierte Prüfung mit dem vorhandenen Validator:
   ```bash
   python scripts/validate_json.py contracts/policy_snapshot.schema.json /tmp/heimlern_snapshot.json
   ```
   Alternativ validiert `just schema:validate` sowohl Snapshot als auch
   Feedback-Beispiel in einem Lauf.
3. Der Validator führt neben der JSON-Schema-Prüfung zusätzliche Konsistenz-
   Checks aus, z. B. dass die Längen von `arms`, `counts` und `values`
   übereinstimmen.

Bei Validierungsfehlern bricht der Befehl mit Exit-Code ≠ 0 ab und nennt das
betroffene Feld. Dadurch lassen sich Snapshots vor Deployments oder vor dem
Import aus Drittquellen absichern.

## 3. Snapshot anwenden

1. Validiertes JSON in die Policy laden, typischerweise beim Start eines
   Dienstes:
   ```rust
   let snapshot = serde_json::from_str::<serde_json::Value>(&json_str)?;
   bandit.load(snapshot);
   ```
2. Anschließend kann die Policy unmittelbar wieder Entscheidungen treffen und
   Feedback verarbeiten (`decide`/`feedback`).
3. Im CI/CD-Kontext empfiehlt es sich, nach erfolgreicher Validierung den
   Snapshot unter Versionskontrolle zu taggen oder in ein Artefakt-Repository
   hochzuladen, um den Stand nachverfolgen zu können.

## Validator-Workflow für JSONL-Samples

Beispiel-Events (z. B. `data/samples/aussensensor.jsonl`) enthalten mehrere
Dokumente im JSON-Lines-Format. Sie können direkt gegen den
[Außensensor-Contract](../contracts/aussen_event.schema.json) geprüft werden:

```bash
python scripts/validate_json.py contracts/aussen_event.schema.json data/samples/aussensensor.jsonl
```

Der Validator liest jede Zeile, validiert sie einzeln und gibt ein ✓ je Zeile
aus. Fehler werden mit Zeilennummer ausgewiesen, sodass defekte Events schnell
gefunden und korrigiert werden können. Anschließend lassen sich die gleichen
Samples z. B. über das Beispiel `ingest_events` einlesen:

```bash
cargo run -p heimlern-core --example ingest_events -- data/samples/aussensensor.jsonl
```

Damit steht für Sensorsamples der gleiche Qualitäts-Check zur Verfügung wie
für Policy-Snapshots.
```
```

### 📄 merges/heimlern_merge_2510262237__docs_adr.md

**Größe:** 1 KB | **md5:** `85899d4e8387db25ae93c5b6ee83e182`

```markdown
### 📄 docs/adr/0001-policy-explainability.md

**Größe:** 441 B | **md5:** `3db85bdaaa67b3e98d3352ba948b16db`

```markdown
# ADR-0001: Policy/Decision als Rust-Lib, Explainability (`why`)
Status: Accepted
Date: 2025-10-12

## Kontext
Entscheidungen sollen reproduzierbar und begründet sein.

## Entscheidung
- Rust-Lib (heimlern-core + -bandits).
- Jede Decision hat `action`, `score`, `why`.

## Konsequenzen
- Klare Schnittstelle zu hausKI; leicht testbar.
- `why` wird im Leitstand angezeigt.

## Alternativen
- Ad-hoc Heuristiken ohne Begründung: verworfen.
```

### 📄 docs/adr/0002-policy-snapshot-persistenz.md

**Größe:** 373 B | **md5:** `2aa6b08424d3ec705dc98b1cd24acc12`

```markdown
# ADR-0002: Policy-Snapshot-Persistenz (SQLite via hausKI)
Status: Accepted
Date: 2025-10-12

## Kontext
Lernen benötigt zustandsvolle Parameter.

## Entscheidung
- Snapshots (JSON) persistieren; Laden/Speichern via hausKI.

## Konsequenzen
- Wiederaufnahme nach Neustart möglich.
- Migrationen über Versionsfeld.

## Alternativen
- Nur In-Memory: Verlust bei Neustart.
```

### 📄 docs/adr/README.md

**Größe:** 253 B | **md5:** `e407c51b96ab4a129adcc0c9d1f71441`

```markdown
# Architekturentscheidungsaufzeichnungen (ADR)

## Index

- [ADR-0001: Policy/Decision als Rust-Lib, Explainability (`why`)](0001-policy-explainability.md)
- [ADR-0002: Policy-Snapshot-Persistenz (SQLite via hausKI)](0002-policy-snapshot-persistenz.md)
```
```

### 📄 merges/heimlern_merge_2510262237__index.md

**Größe:** 15 KB | **md5:** `4b6ba059eb30b4c7ba3532902601fccb`

```markdown
# Ordner-Merge: heimlern

**Zeitpunkt:** 2025-10-26 22:37
**Quelle:** `/home/alex/repos/heimlern`
**Dateien (gefunden):** 37
**Gesamtgröße (roh):** 44 KB

**Exclude:** ['.gitignore']

## 📁 Struktur

- heimlern/
  - .gitignore
  - .hauski-reports
  - CONTRIBUTING.md
  - Cargo.lock
  - Cargo.toml
  - Justfile
  - LICENSE
  - README.md
  - requirements-tools.txt
  - tests/
    - fixtures/
      - .gitkeep
      - aussen.jsonl
  - docs/
    - policies-os-context.md
    - policy-lifecycle.md
    - adr/
      - 0001-policy-explainability.md
      - 0002-policy-snapshot-persistenz.md
      - README.md
  - .github/
    - workflows/
      - ci-rust.yml
      - ci.yml
      - contracts.yml
      - validate-aussen-samples.yml
      - validate-aussen.yml
  - .git/
    - FETCH_HEAD
    - HEAD
    - ORIG_HEAD
    - config
    - index
    - packed-refs
    - hooks/
      - pre-push
    - refs/
      - remotes/
        - origin/
          - alert-autofix-2
          - docs-improve-event-documentation
          - fehlerbehebung-im-repo
          - fix-bandit-logic-and-add-tests
          - main
          - refactor-reward-calculation
          - fix/
            - fehlerfrei-check
          - codex/
            - add-adr-0001-and-adr-0002-documents
            - add-cached-rust-ci-workflow-and-tests
            - add-contracts-and-validation-scripts
            - add-documentation-for-policy-lifecycle
            - add-github-actions-workflow-for-validation
            - add-missing-workflow-and-event-files
            - add-policy-lifecycle-documentation-and-workflow
            - find-code-errors
            - find-errors-in-the-code
            - find-errors-in-the-code-msa0re
            - fix-build-error-in-lib.rs
            - fix-syntax-errors-in-lib.rs
            - fix-validate-aussen.yml-workflow-failures
            - harden-epsilon-handling-and-add-decide-example
            - update-policies-for-os-context
            - verify-documentation-completeness
      - tags/
      - heads/
        - main
        - backup/
          - main-20251017-182445
          - main-20251018-090526
          - main-20251021-124304
          - main-20251023-070602
          - main-20251026-162049
    - logs/
      - HEAD
      - refs/
        - remotes/
          - origin/
            - alert-autofix-2
            - docs-improve-event-documentation
            - fehlerbehebung-im-repo
            - fix-bandit-logic-and-add-tests
            - main
            - refactor-reward-calculation
            - fix/
              - fehlerfrei-check
            - codex/
              - add-adr-0001-and-adr-0002-documents
              - add-cached-rust-ci-workflow-and-tests
              - add-contracts-and-validation-scripts
              - add-documentation-for-policy-lifecycle
              - add-github-actions-workflow-for-validation
              - add-missing-workflow-and-event-files
              - add-policy-lifecycle-documentation-and-workflow
              - find-code-errors
              - find-errors-in-the-code
              - find-errors-in-the-code-msa0re
              - fix-build-error-in-lib.rs
              - fix-syntax-errors-in-lib.rs
              - fix-validate-aussen.yml-workflow-failures
              - harden-epsilon-handling-and-add-decide-example
              - update-policies-for-os-context
              - verify-documentation-completeness
        - heads/
          - main
          - backup/
            - main-20251017-182445
            - main-20251018-090526
            - main-20251021-124304
            - main-20251023-070602
            - main-20251026-162049
    - objects/
      - de/
        - 4c0423c9b0096c41d6460600d1c115bda976c9
      - c7/
        - 5e3c49e1c78021a1a6de7f14c39f6d83b78de9
      - 87/
        - 30ae26584d87fe04a7894ce0f80a1ce856869f
        - ec078c6679540bbd09c4355938e9ed3a5e1e49
      - ad/
        - c9ea28221387eef0fd6167f6ada14dcea23570
      - 89/
        - a5d9d56327e2b627359508aa922f406a14afab
      - 6b/
        - 4ae090b88c66721e23e3445917d1f5996618e4
      - 54/
        - 1a675882268106c89eeaebb0766fcb9b82bffd
      - 17/
        - 435274f59d73a1d6d884088c9136eed089b7d6
      - a8/
        - db0a3b9014bf77aac4e72c688230f7f6879cba
      - fd/
        - 2f80638b32137c6ebba6203749fd95a1eb8672
        - 68cc5a044ee7f6180de4c6b268012aa91859fc
      - cc/
        - 2150f009879b9e2e91b751296018046523fbc8
      - b2/
        - 8c0dc7aff5cbe8807f86f913b1020f6b0c5627
        - f535a70d83eac1c899af36bb49f43a42dc944f
      - 14/
        - 3012b00e243492fb70d408f0475071b6270bd1
      - f9/
        - 634db084ec2c3d565aaf70eb635514bab5312e
      - 6a/
        - 1a61d028a9023a70e35bd05e87294e44841d8a
        - 8a88c218b8b07948def2ebd12bff3a4d75f4e2
      - 3e/
        - 155bb69acf964013c15d5152a3d451ced17199
        - 64930bf15b692555ca7deac352d2d496d53fcd
        - 98df14d5bfb499e78c4804bd9f805cd6e57283
      - 01/
        - b2c165765497625de9b2a2d019f035e6d9b3c7
      - 13/
        - 3cfd2fe43a2087add59db2578d8236c896656d
      - 56/
        - 67ac24485c80b772e61aef9040f1a39a78b39e
      - 9e/
        - a85789f5cf1bd3082cc0641d208e8e4f75d0dc
        - f115039ebb5eb55c5f2a2392340106813811eb
      - 2f/
        - 7896d1d1365eafb0da03d9fe456fac81408487
      - 8d/
        - c4129c08ad481d3b2df3a969edeaaf1bbb80e6
      - pack/
        - pack-1a5e4d0b13c5d7cd8e8fa88ccac78650c78b1033.idx
        - pack-1a5e4d0b13c5d7cd8e8fa88ccac78650c78b1033.pack
      - 9a/
        - 048552af3d91b04b484add69d5ef68766749e9
      - f1/
        - cb042b1481f3aad2ac7d2e5bcffaa023e0fc42
        - e1dfe054c6aa1164e48e476556076ba181ad6d
      - 9b/
        - 23ab534f1a9a72d876f089176d072962d10f71
      - c3/
        - 1a09eded75a10ca595c1f140cde5e8a7d5cefb
      - cb/
        - 0fd915ef699324fb6d94c6a7a6c52522c8f7b1
      - bb/
        - d315277b8d9248c7857aecfd6cb7f9a1a054bf
      - 94/
        - 2abbdfb0a33005ce3f10481fcd37dc8b636ec3
      - 18/
        - 66874041cb814cc62dc385980ec651e2b61592
        - e93364a666cf7919a8c35434f96e99aba06ea6
        - fcabf0832a36f09d493637e02e8f25ee63b150
      - df/
        - 08a61a1c79a17d20343afc95d68a6dd28f1070
      - 3b/
        - 89661312f4b58d68f14113ed8b117fad3ba6ff
      - e4/
        - 10fed35382eea8781c64de1f5594f1feccb677
      - d4/
        - 618f5cce579d2e053fbe8d877645e99499608f
      - 20/
        - d9cc369d9d4c90ff79e639860b2e2ac4b6f4d9
      - 19/
        - 7fd682fb50b11b66f720eff6bbe48bd9ada3b5
        - 846b330b6b1dba9aeac296be45825d0af5c453
      - ff/
        - ee584d4326bba6121d2a6f52806c2da2f5b952
      - c4/
        - 3b9991e52683ebad71e769bc5302cd85a6ba63
        - 7ec264a91ad657860f2deb0ebeb0c203e007a4
      - fc/
        - 2a15c434e3add52eb5d9539bd0befd1a590084
        - c9f917b416a5856a12dd0bac769d727dab2e0b
      - ec/
        - 09bd623040e3982c7c1b5cf1536e7413ecf95e
      - e6/
        - 4e540de9b0322a2a4602b30b9c82b1e13fbfb1
      - 50/
        - a5d4d16531ea7e7766ab53f957d15c11ad1cff
      - 5b/
        - 0cfc411270f78daf5768f674dbc61c38a0a6f9
      - 5d/
        - 550a3bde39e124eb546a16cc8c00e3aabafb1d
        - a55307d6aedb9274165a45d82cb9fe70d1a913
        - ed214a543e54ab517076c013ed0c3360569688
      - 7a/
        - 6635b8a8acbae1d946605af0e6cc9adafb388e
      - 16/
        - 9ffcb7297ef4d28f3ce6315c30fdb23a1d166d
      - 28/
        - cc39a51ba2f7b1c37f763bb63554b2d09831bc
      - fb/
        - e53ad8832a50c146f03f9605bc7d89c69384ec
      - 4c/
        - 8a4fb5803d59479cd447f0f92394c4206ef5e7
      - ed/
        - 742bee7396ec6c72a20acaeaf4334da2348193
      - 53/
        - 3b2755a239b7695ec225cfcbf5919c60b661f6
      - 46/
        - 070e16a2a53322b173307a3edb926ae022dade
      - d9/
        - 1381297b9ec58d603f001dc98e8e2c5f48d2ef
      - 6f/
        - 22d1b1ce5861b6b04c12ba58030fa8265b4911
      - 03/
        - bb0cca586c3f2ea1e47fe513ec4ee896c3aede
      - 2b/
        - 63447e93ec27605a1512918b667b1421766380
      - 75/
        - ba1ab190d89fea56d7f568764de70957b079e9
      - 32/
        - fb6f2ff1cde77e26b1c2cfbecbd034a46d017b
      - 63/
        - 9659017d39fdb9ea184f372d95d734df2c6862
      - d6/
        - 9b2786843e78ab38c2177dfceba9d3117c9e83
      - 80/
        - 2fe5da93e76e87d80b0e296564c3cc2bbf4840
      - 2a/
        - 53909aed93a541989a845c0c56499eeeaf5e3c
      - 3a/
        - 1ab85ac0c6368ce65e9ce48f2047279d5ceae6
      - e2/
        - d69d6a85fcb8502e648015295f5c313ce75633
      - b3/
        - 9d942c0ef7a732b188ebdddf9d8a459eb2b4e8
      - a5/
        - 6192472a1ebf2af36cd949345b71218c593be4
        - 99c70857153012b1bac1f965325211f5d5b3f2
      - c8/
        - 69c16f8625fee59bd9d0361c03a4b0adf2cea6
      - 6e/
        - 8e4ca935505fb991ab98c308ee2517418c9838
      - 9c/
        - d221e2507a204e6ee65c468ba989dbf32e168d
      - 10/
        - 7d5bc256a898ea0fa67d8719eca112fba038dc
      - 1c/
        - 46a89b42a544bfa8da682c811bb4a71e460fa6
      - 55/
        - a0904c93d795bd9c21cd46b709fe1fa85d8608
      - b1/
        - 0e2712d41ec33d8b8fc0fa0a3ab3ef2f82a406
      - 85/
        - 73d203b14f24b6e0067ac8c36a9ae84c031115
        - bd28838bdfab4b28be06c537f23e835d777e7a
      - 21/
        - 87c5e35bb7ec918716d2fefe28a75fb1015c23
      - da/
        - 08ad051a2c5c611d606f431a7c1ceb2388301f
        - 0e559ce8b96f5de92fb0705ac728e55962c897
      - 3d/
        - 1c301ca76aa1beb78b611fee3838b3598e8ec8
      - 1e/
        - 1ddf4d428c049ab4423bac24abc15989863042
        - d3531abbd42392497ae151ad7211f00664f70a
      - 1f/
        - 33320bb30c4d3432b2b7199d6ab88d82c14677
        - 992a77f14ebe5ca2e23d2bc14942e20667a052
      - dc/
        - a5c622efca85f947628f13d7617f29d1f8c8d1
      - 04/
        - 83d72b7a8abe93b1e4f5fab7297414b8e54c82
        - 974d99f1a6308e589dbaa1cdb2b298adc2d3a2
        - e1931c6c55ce1b4c49484d146cd89d70a93e37
        - f1dae7a2ca4fb554426f946de8a10f0e1d086e
      - 8c/
        - 068e9421f7c184e66fab2c63d030c1046f748d
        - f528bef7311ee195c246617764b7fa023e0ac6
      - 7f/
        - 33f8a62457700cfa7a7bf262bd9dd7d08c0c19
        - b479d0499bbaf1af7f390df30633d206501275
      - 25/
        - f2ed3e933f211e8d448b10850c3cbf1548b571
      - c0/
        - 3b3efa9bc1f6bc04d7106090406e264e87a515
      - bf/
        - a9a3c4449c57dd3eb743ea30160e273605db2b
      - 05/
        - 24bfa6cc66a5d176d66c476f248a6a89d18689
      - 65/
        - 9233647e218578eb29bff7889211a674e503bd
      - ea/
        - 2a5eaec4f70c33e7f194051edbf87575a6111c
        - 5eb29f0ddeef537b5aeb507165d1c4e11d7188
      - 51/
        - 7c5e6654730975500955e8cca2d9265231e956
        - 88aa12048ff118b09051a959ac528a49b6789f
        - d746e67e3f0b5293bb3b51b681e35950d3fd14
      - ee/
        - 926e0f6d465f71e4504f0d0edf7021a3262914
      - e7/
        - f861f275f894fe6e783d093e3e1b4003e43903
      - 9f/
        - 62ca5059b0aee771b60f48ec411acd63db7396
        - 970225adb6a6ada5c22d46d02684b6b0f5525e
      - 36/
        - 9e15a9ecbec8d099b0266dc1106ba869563bfd
      - 93/
        - c49af6063a40c59832e3421a57e1f269308ff9
      - info/
      - 86/
        - 37a20cb40703b662edd21bfd74e05bbf562931
      - 15/
        - 9296874bf256cfe7c089f3b54ade1bfcbcf824
      - b6/
        - 4e9f287a803fee8ec4971a15f151978a21dc42
        - 823db7622482cd39da029189ec6c70a1dff4d1
        - b0e7739b62ea6e9c23db5b10d1bcb16d45d325
        - b91c1972c0878b98e44219d9c315aeb1085af7
      - 40/
        - acaa567aeb215d1c1d6134ec1892cb4f09f63b
      - 88/
        - 1d66b58d092a24fa65719ae4d78c1b2f63cade
      - 4e/
        - 8f50ef6fcd1ce14b5f1d696b49cbb62892d3ec
      - 09/
        - 3f40de5431baef9d61b9b6c722e5ec52bee46a
        - ad33aa2ee414f27ae8bdff6cb6a382f17e9ac6
      - 0b/
        - 34ac6dddb45abdcc0dab3a7d7c31dc0a022cb7
        - fd504b4c1d47d5d24d644c7f8cab4aed80ad34
      - 22/
        - e8262c92b72b7009ba23b79fff818f2b6c2c63
      - 33/
        - 3f7f286ef76171eff1f7a887aa0a415f6dbee8
        - 76aea0d490f31ac4cad467fba13ec88c6584e3
      - ef/
        - 55c348bb37b4ac902e04e0ba5b4c1ef3d5ae53
        - ab909afdb4e0af60198d03e0fcb56108ed385c
      - a2/
        - 00982721b70cd2f23c118b33aac9d245c9c4bd
  - merges/
    - heimlern_merge_2510262237__index.md
  - crates/
    - heimlern-core/
      - Cargo.toml
      - src/
        - event.rs
        - lib.rs
      - examples/
        - ingest_events.rs
    - heimlern-bandits/
      - Cargo.toml
      - src/
        - error.rs
        - lib.rs
      - examples/
        - decide.rs
        - integrate_hauski.rs
  - scripts/
    - examples.py
    - validate_json.py
  - data/
    - samples/
      - .gitkeep
      - aussensensor.jsonl
  - contracts/
    - README.md
    - aussen_event.schema.json
    - policy_feedback.schema.json
    - policy_snapshot.schema.json

## 📦 Inhalte (Chunks)

- .gitignore → `heimlern_merge_2510262237__root.md`
- CONTRIBUTING.md → `heimlern_merge_2510262237__root.md`
- Cargo.lock → `heimlern_merge_2510262237__root.md`
- Cargo.toml → `heimlern_merge_2510262237__root.md`
- Justfile → `heimlern_merge_2510262237__root.md`
- LICENSE → `heimlern_merge_2510262237__root.md`
- README.md → `heimlern_merge_2510262237__root.md`
- requirements-tools.txt → `heimlern_merge_2510262237__root.md`
- tests/fixtures/.gitkeep → `heimlern_merge_2510262237__tests_fixtures.md`
- tests/fixtures/aussen.jsonl → `heimlern_merge_2510262237__tests_fixtures.md`
- docs/policies-os-context.md → `heimlern_merge_2510262237__docs.md`
- docs/policy-lifecycle.md → `heimlern_merge_2510262237__docs.md`
- docs/adr/0001-policy-explainability.md → `heimlern_merge_2510262237__docs_adr.md`
- docs/adr/0002-policy-snapshot-persistenz.md → `heimlern_merge_2510262237__docs_adr.md`
- docs/adr/README.md → `heimlern_merge_2510262237__docs_adr.md`
- .github/workflows/ci-rust.yml → `heimlern_merge_2510262237__.github_workflows.md`
- .github/workflows/ci.yml → `heimlern_merge_2510262237__.github_workflows.md`
- .github/workflows/contracts.yml → `heimlern_merge_2510262237__.github_workflows.md`
- .github/workflows/validate-aussen-samples.yml → `heimlern_merge_2510262237__.github_workflows.md`
- .github/workflows/validate-aussen.yml → `heimlern_merge_2510262237__.github_workflows.md`
- crates/heimlern-core/Cargo.toml → `heimlern_merge_2510262237__crates_heimlern-core.md`
- crates/heimlern-core/src/event.rs → `heimlern_merge_2510262237__crates_heimlern-core_src.md`
- crates/heimlern-core/src/lib.rs → `heimlern_merge_2510262237__crates_heimlern-core_src.md`
- crates/heimlern-core/examples/ingest_events.rs → `heimlern_merge_2510262237__crates_heimlern-core_examples.md`
- crates/heimlern-bandits/Cargo.toml → `heimlern_merge_2510262237__crates_heimlern-bandits.md`
- crates/heimlern-bandits/src/error.rs → `heimlern_merge_2510262237__crates_heimlern-bandits_src.md`
- crates/heimlern-bandits/src/lib.rs → `heimlern_merge_2510262237__crates_heimlern-bandits_src.md`
- crates/heimlern-bandits/examples/decide.rs → `heimlern_merge_2510262237__crates_heimlern-bandits_examples.md`
- crates/heimlern-bandits/examples/integrate_hauski.rs → `heimlern_merge_2510262237__crates_heimlern-bandits_examples.md`
- scripts/examples.py → `heimlern_merge_2510262237__scripts.md`
- scripts/validate_json.py → `heimlern_merge_2510262237__scripts.md`
- data/samples/.gitkeep → `heimlern_merge_2510262237__data_samples.md`
- data/samples/aussensensor.jsonl → `heimlern_merge_2510262237__data_samples.md`
- contracts/README.md → `heimlern_merge_2510262237__contracts.md`
- contracts/aussen_event.schema.json → `heimlern_merge_2510262237__contracts.md`
- contracts/policy_feedback.schema.json → `heimlern_merge_2510262237__contracts.md`
- contracts/policy_snapshot.schema.json → `heimlern_merge_2510262237__contracts.md`
```

### 📄 merges/heimlern_merge_2510262237__part001.md

**Größe:** 43 B | **md5:** `ad150e6cdda3920dbef4d54c92745d83`

```markdown
<!-- chunk:1 created:2025-10-26 22:37 -->
```

### 📄 merges/heimlern_merge_2510262237__root.md

**Größe:** 12 KB | **md5:** `8050525b82c3e8511bfe690d80d5771a`

```markdown
### 📄 .gitignore

**Größe:** 29 B | **md5:** `98bcb2d7445831c7821b9c9f9234a0a0`

```plaintext
target/

.venv/
__pycache__/
```

### 📄 CONTRIBUTING.md

**Größe:** 570 B | **md5:** `4b790aec5719a4de163dcdf25ceda143`

```markdown
# Beitrag leisten

Danke für deinen Beitrag zu **heimlern**.

## Setup
```bash
git clone https://github.com/heimgewebe/heimlern.git
cd heimlern
cargo build --workspace
cargo test  --workspace
```

## Coding-Guidelines (Kurz)
- Keine `unwrap()`/`expect()` in Bibliothekscode – stattdessen `Result` zurückgeben.
- Unit-Tests in jeder Crate für Kernstrukturen.
- PRs gegen `main` mit „grüner“ CI.

## Commit-Stil
- Präfixe: `feat:`, `fix:`, `docs:`, `ci:`, `refactor:`, `test:`, `chore:`

## Lizenz
Durch das Einreichen eines PR stimmst du der Projektlizenz zu.
```

### 📄 Cargo.lock

**Größe:** 5 KB | **md5:** `a5f76c2e3635c827149dfb4eab097503`

```plaintext
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "cfg-if"
version = "1.0.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2fd1289c04a9ea8cb22300a459a72a385d7c73d3259e2ed7dcb2af674838cfa9"

[[package]]
name = "getrandom"
version = "0.2.16"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "335ff9f135e4384c8150d6f27c6daed433577f86b4750418338c01a1a2528592"
dependencies = [
 "cfg-if",
 "libc",
 "wasi",
]

[[package]]
name = "heimlern-bandits"
version = "0.1.0"
dependencies = [
 "heimlern-core",
 "rand",
 "serde",
 "serde_json",
 "thiserror",
]

[[package]]
name = "heimlern-core"
version = "0.1.0"
dependencies = [
 "serde",
 "serde_json",
]

[[package]]
name = "itoa"
version = "1.0.15"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4a5f13b858c8d314ee3e8f639011f7ccefe71f97f96e50151fb991f267928e2c"

[[package]]
name = "libc"
version = "0.2.177"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2874a2af47a2325c2001a6e6fad9b16a53b802102b528163885171cf92b15976"

[[package]]
name = "memchr"
version = "2.7.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f52b00d39961fc5b2736ea853c9cc86238e165017a493d1d5c8eac6bdc4cc273"

[[package]]
name = "ppv-lite86"
version = "0.2.21"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "85eae3c4ed2f50dcfe72643da4befc30deadb458a9b590d720cde2f2b1e97da9"
dependencies = [
 "zerocopy",
]

[[package]]
name = "proc-macro2"
version = "1.0.101"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "89ae43fd86e4158d6db51ad8e2b80f313af9cc74f5c0e03ccb87de09998732de"
dependencies = [
 "unicode-ident",
]

[[package]]
name = "quote"
version = "1.0.41"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ce25767e7b499d1b604768e7cde645d14cc8584231ea6b295e9c9eb22c02e1d1"
dependencies = [
 "proc-macro2",
]

[[package]]
name = "rand"
version = "0.8.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "34af8d1a0e25924bc5b7c43c079c942339d8f0a8b57c39049bef581b46327404"
dependencies = [
 "libc",
 "rand_chacha",
 "rand_core",
]

[[package]]
name = "rand_chacha"
version = "0.3.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e6c10a63a0fa32252be49d21e7709d4d4baf8d231c2dbce1eaa8141b9b127d88"
dependencies = [
 "ppv-lite86",
 "rand_core",
]

[[package]]
name = "rand_core"
version = "0.6.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ec0be4795e2f6a28069bec0b5ff3e2ac9bafc99e6a9a7dc3547996c5c816922c"
dependencies = [
 "getrandom",
]

[[package]]
name = "ryu"
version = "1.0.20"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "28d3b2b1366ec20994f1fd18c3c594f05c5dd4bc44d8bb0c1c632c8d6829481f"

[[package]]
name = "serde"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9a8e94ea7f378bd32cbbd37198a4a91436180c5bb472411e48b5ec2e2124ae9e"
dependencies = [
 "serde_core",
 "serde_derive",
]

[[package]]
name = "serde_core"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "41d385c7d4ca58e59fc732af25c3983b67ac852c1a25000afe1175de458b67ad"
dependencies = [
 "serde_derive",
]

[[package]]
name = "serde_derive"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d540f220d3187173da220f885ab66608367b6574e925011a9353e4badda91d79"
dependencies = [
 "proc-macro2",
 "quote",
 "syn",
]

[[package]]
name = "serde_json"
version = "1.0.145"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "402a6f66d8c709116cf22f558eab210f5a50187f702eb4d7e5ef38d9a7f1c79c"
dependencies = [
 "itoa",
 "memchr",
 "ryu",
 "serde",
 "serde_core",
]

[[package]]
name = "syn"
version = "2.0.106"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ede7c438028d4436d71104916910f5bb611972c5cfd7f89b8300a8186e6fada6"
dependencies = [
 "proc-macro2",
 "quote",
 "unicode-ident",
]

[[package]]
name = "thiserror"
version = "1.0.69"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b6aaf5339b578ea85b50e080feb250a3e8ae8cfcdff9a461c9ec2904bc923f52"
dependencies = [
 "thiserror-impl",
]

[[package]]
name = "thiserror-impl"
version = "1.0.69"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4fee6c4efc90059e10f81e6d42c60a18f76588c3d74cb83a0b242a2b6c7504c1"
dependencies = [
 "proc-macro2",
 "quote",
 "syn",
]

[[package]]
name = "unicode-ident"
version = "1.0.19"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f63a545481291138910575129486daeaf8ac54aee4387fe7906919f7830c7d9d"

[[package]]
name = "wasi"
version = "0.11.1+wasi-snapshot-preview1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ccf3ec651a847eb01de73ccad15eb7d99f80485de043efb2f370cd654f4ea44b"

[[package]]
name = "zerocopy"
version = "0.8.27"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0894878a5fa3edfd6da3f88c4805f4c8558e2b996227a3d864f47fe11e38282c"
dependencies = [
 "zerocopy-derive",
]

[[package]]
name = "zerocopy-derive"
version = "0.8.27"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "88d2b8d9c68ad2b9e4340d7832716a4d21a22a1154777ad56ea55c51a9cf3831"
dependencies = [
 "proc-macro2",
 "quote",
 "syn",
]
```

### 📄 Cargo.toml

**Größe:** 152 B | **md5:** `effe0f96dfcfefeb1240c84457bdf70f`

```toml
[workspace]
resolver = "2"
members = ["crates/heimlern-core","crates/heimlern-bandits"]

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
```

### 📄 Justfile

**Größe:** 949 B | **md5:** `dfec7b7b046f1428611639b125ce350e`

```plaintext
set shell := ["bash","-eu","-o","pipefail","-c"]

default: schema:validate

# ---- Python venv für Tools (jsonschema) ----
venv:
	@test -d .venv || python3 -m venv .venv
	. .venv/bin/activate && python -m pip install --upgrade pip
	. .venv/bin/activate && pip install -r requirements-tools.txt

# ---- Beispiele schreiben ----
snapshot:example:
	. .venv/bin/activate || true
	python3 scripts/examples.py
	@ls -l /tmp/heimlern_snapshot.json

feedback:example:
	. .venv/bin/activate || true
	python3 scripts/examples.py
	@ls -l /tmp/heimlern_feedback.json

# ---- Validierung ----
schema:validate: venv
	. .venv/bin/activate && python scripts/examples.py
	. .venv/bin/activate && python scripts/validate_json.py contracts/policy_snapshot.schema.json /tmp/heimlern_snapshot.json
	. .venv/bin/activate && python scripts/validate_json.py contracts/policy_feedback.schema.json /tmp/heimlern_feedback.json
	@echo "✓ alle Beispiel-Dokumente sind valide"
```

### 📄 LICENSE

**Größe:** 236 B | **md5:** `add9e052397f3389aad535e258d9ddf9`

```plaintext
MIT License
Copyright (c) 2025 heimgewebe
Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction...
```

### 📄 README.md

**Größe:** 4 KB | **md5:** `bea089ac3698fa1e7b56a588e00de24b`

```markdown
# heimlern

[![rust (cached)](https://github.com/heimgewebe/heimlern/actions/workflows/ci-rust.yml/badge.svg)](https://github.com/heimgewebe/heimlern/actions/workflows/ci-rust.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Siehe auch: **Policy-Lifecycle**: `docs/policy-lifecycle.md` und **Contracts** in `contracts/`.

`heimlern` ist ein kleines Experimentierfeld für lernfähige Policies im häuslichen Umfeld. Das Repository besteht aus einem schlanken Kern mit gemeinsam genutzten Traits sowie einer Beispiel-Implementierung eines Bandit-Agenten, die zusammen zeigen, wie Erinnerungs-Policies modelliert, ausgeführt und persistiert werden können.

## Inhaltsverzeichnis

1. [Schnellstart](#schnellstart)
2. [Architekturüberblick](#architekturüberblick)
3. [Crates](#crates)
4. [Beispiel ausführen](#beispiel-ausführen)
5. [Weiterführende Dokumentation](#weiterführende-dokumentation)
6. [Installation / Entwicklung](#installation--entwicklung)
7. [Beitragen](#beitragen)

## Schnellstart

Voraussetzungen:

* [Rust](https://www.rust-lang.org/tools/install) ab Version 1.70

Repository klonen und Beispiel ausführen:

```bash
git clone https://github.com/heimgewebe/heimlern.git
cd heimlern
cargo run --example integrate_hauski
```

## Architekturüberblick

* **Policies** sind Strategien, die auf Basis eines `Context` Entscheidungen (`Decision`) treffen und über Feedback lernen.
* **Snapshots** sichern und laden den internen Zustand einer Policy als JSON, sodass Agenten zwischen Sessions fortgeführt werden können.
* **Bandit-Implementierungen** kombinieren Exploration (Zufall) und Exploitation (Heuristik), um passende Erinnerungs-Slots zu wählen.

Die zentralen Entwurfsentscheidungen sind in den [Architecture Decision Records](docs/adr/README.md) dokumentiert.

## Crates

| Crate | Zweck |
| --- | --- |
| [`heimlern-core`](crates/heimlern-core) | Definiert die Basistypen `Context`, `Decision` sowie das `Policy`-Trait und beschreibt das JSON-basierte Snapshot-Interface. |
| [`heimlern-bandits`](crates/heimlern-bandits) | Enthält den Beispielagenten `RemindBandit`, der über ε-greedy Exploration Erinnerungs-Slots auswählt. |

## Beispiel ausführen

Das Beispiel `integrate_hauski` simuliert einen einfachen Ablauf: Es erstellt Kontextdaten, ruft den Bandit zur Entscheidungsfindung auf und demonstriert den Snapshot-Mechanismus.

```bash
cargo run --example integrate_hauski
```

Die Ausgabe enthält die gewählte Aktion, den Grund (Exploration oder Heuristik) und den in JSON serialisierten Kontext.

Für schnelle Smoke-Tests des Banditen eignet sich außerdem das Beispiel `decide`. Es liest einen Kontext aus `stdin` (oder verwendet Defaults) und gibt die Entscheidung als JSON im [Policy-Schema](crates/heimlern-core/src/policy.rs) aus:

```bash
echo '{}' | cargo run -p heimlern-bandits --example decide
```

Ersetze `{}` durch einen gewünschten Kontext, um andere Slots oder Heuristiken zu prüfen.

## Weiterführende Dokumentation

* [ADR-Index](docs/adr/README.md) – Übersicht und Motivation hinter den Architekturentscheidungen.
* Policy-Lifecycle: `docs/policy-lifecycle.md`
* Inline-Rustdocs in den Crates (`cargo doc --open`) erläutern Strukturen, Traits und das Snapshot-Format im Detail.

### Beispiel: Außensensor-Events grob scoren

```bash
# kompiliert und liest JSONL aus Datei oder stdin
cargo run -p heimlern-core --example ingest_events -- data/samples/aussensensor.jsonl
```
Die Ausgabe listet pro Zeile einen Score (0..1) und den Titel (falls vorhanden).

## Installation / Entwicklung

### Anforderungen
- Rust (stable)
- Cargo

### Schnellstart
```bash
git clone https://github.com/heimgewebe/heimlern.git
cd heimlern
cargo build --workspace
cargo test  --workspace
```

## Beitragen
Siehe [CONTRIBUTING.md](CONTRIBUTING.md).
```

### 📄 requirements-tools.txt

**Größe:** 20 B | **md5:** `d862a99e1534a974ea0523f43a48a910`

```plaintext
jsonschema>=4.22,<5
```
```

### 📄 merges/heimlern_merge_2510262237__scripts.md

**Größe:** 4 KB | **md5:** `1da03d77687b3147d9af4845ef9f1a33`

```markdown
### 📄 scripts/examples.py

**Größe:** 1 KB | **md5:** `7847620050da59acbf01323e4d6fe208`

```python
#!/usr/bin/env python3
"""
Schreibt minimal-belegte Beispiel-Dokumente in /tmp für schnelles Testen.
"""
from __future__ import annotations

import json
import time
from datetime import datetime, timezone
from pathlib import Path


def iso_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def write(path: Path, obj) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(obj, handle, ensure_ascii=False, indent=2)
    print("→", path)


def main() -> None:
    snapshot = {
        "version": "0.1.0",
        "policy_id": "example-policy",
        "ts": iso_now(),
        "arms": ["a", "b", "c"],
        "counts": [10, 5, 2],
        "values": [0.12, 0.18, 0.05],
        "epsilon": 0.1,
        "seed": 42,
    }
    feedback = {
        "version": "0.1.0",
        "policy_id": "example-policy",
        "ts": iso_now(),
        "decision_id": "dec-" + str(int(time.time())),
        "reward": 1.0,
        "notes": "first feedback",
    }

    write(Path("/tmp/heimlern_snapshot.json"), snapshot)
    write(Path("/tmp/heimlern_feedback.json"), feedback)


if __name__ == "__main__":
    main()
```

### 📄 scripts/validate_json.py

**Größe:** 3 KB | **md5:** `57248893a205fc3d7d2b83bb71f5da30`

```python
#!/usr/bin/env python3
"""
Kleiner Validator für heimlern-Contracts.
Verwendung:
  python scripts/validate_json.py contracts/policy_snapshot.schema.json /path/to/doc.json
"""
from __future__ import annotations

import json
import pathlib
import sys
from typing import Any, Sequence

from jsonschema import Draft202012Validator, RefResolver


class ContractError(ValueError):
    pass


def _extra_checks(schema_path: pathlib.Path, data: Any) -> None:
    if schema_path.name == "policy_snapshot.schema.json":
        arms = data.get("arms")
        counts = data.get("counts")
        values = data.get("values")
        if isinstance(arms, Sequence) and not isinstance(arms, (str, bytes)):
            expected = len(arms)
            if isinstance(counts, Sequence) and not isinstance(counts, (str, bytes)):
                if len(counts) != expected:
                    raise ContractError("counts length must match arms length")
            if isinstance(values, Sequence) and not isinstance(values, (str, bytes)):
                if len(values) != expected:
                    raise ContractError("values length must match arms length")


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: validate_json.py <schema.json> <document.json>", file=sys.stderr)
        return 2

    schema_path = pathlib.Path(sys.argv[1]).resolve()
    doc_path = pathlib.Path(sys.argv[2]).resolve()

    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    resolver = RefResolver.from_schema(schema)
    validator = Draft202012Validator(schema, resolver=resolver)

    def validate_payload(payload: Any, label: str) -> None:
        validator.validate(payload)
        _extra_checks(schema_path, payload)
        print(f"\u2713 {label} valid against {schema_path.name}")

    if doc_path.suffix == ".jsonl":
        with doc_path.open("r", encoding="utf-8") as handle:
            for idx, raw in enumerate(handle, start=1):
                stripped = raw.strip()
                if not stripped:
                    continue
                try:
                    data = json.loads(stripped)
                except json.JSONDecodeError as exc:  # pragma: no cover - CLI helper
                    raise ContractError(
                        f"line {idx} ist kein valides JSON: {exc}"
                    ) from exc
                validate_payload(data, f"{doc_path.name}:{idx}")
    else:
        data = json.loads(doc_path.read_text(encoding="utf-8"))
        validate_payload(data, doc_path.name)

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # pragma: no cover - CLI helper
        print(f"\u274c Validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
```
```

### 📄 merges/heimlern_merge_2510262237__tests_fixtures.md

**Größe:** 483 B | **md5:** `9e417cdd9201d479d7e7449e34d00081`

```markdown
### 📄 tests/fixtures/.gitkeep

**Größe:** 55 B | **md5:** `1692c32d5661808e33b0a851279f5b20`

```plaintext
# Keep tests/fixtures directory under version control.
```

### 📄 tests/fixtures/aussen.jsonl

**Größe:** 189 B | **md5:** `f0232fa657c8c98c322d956c6d8189b0`

```plaintext
{"type":"link","source":"test","title":"Hello","url":"https://example.org","tags":["demo"],"features":{}}
{"type":"link","source":"test","summary":"No title","url":"https://example.org/2"}
```
```

