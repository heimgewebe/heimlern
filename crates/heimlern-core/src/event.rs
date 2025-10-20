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
