use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Kontrakt-konformer Außensensor-Event (contracts/aussen.event.schema.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AussenEvent {
    /// Eindeutige Kennung des Events.
    pub id: Option<String>,
    /// Typ des Events, entspricht dem JSON-Feld `type`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Quelle oder Herkunft des Events.
    pub source: String,
    /// Optionaler Titel zur Anzeige.
    pub title: Option<String>,
    /// Kurzbeschreibung oder Zusammenfassung.
    pub summary: Option<String>,
    /// Referenz-URL für weiterführende Informationen.
    pub url: Option<String>,
    /// Tags zur Kategorisierung.
    pub tags: Option<Vec<String>>,
    /// ISO-8601-Zeitstempel.
    pub ts: Option<String>,
    /// Beliebige zusätzliche Merkmale.
    pub features: Option<BTreeMap<String, Value>>,
    /// Weitere Metadaten.
    pub meta: Option<BTreeMap<String, Value>>,
}
