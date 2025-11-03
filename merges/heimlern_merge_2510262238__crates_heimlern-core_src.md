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

