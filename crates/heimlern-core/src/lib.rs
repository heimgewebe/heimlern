//! Kern-Typen und Traits für das heimlern-Ökosystem.
//!
//! Die hier definierten Strukturen bilden die Schnittstelle zwischen konkreten
//! Policies und der Umgebung, in der Entscheidungen getroffen und bewertet
//! werden. Alle Typen sind `Serialize`/`Deserialize`, damit sie in JSON-basierte
//! APIs, Persistenzschichten oder Tests eingebettet werden können.

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
    /// Heuristische Bewertung der Aktion im Bereich `0.0..=1.0`.
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
