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
/// `contracts/aussen.event.schema.json` kompatibel ist.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AussenEvent {
    /// Eine eindeutige Kennung für dieses Ereignis, z. B. eine UUID.
    pub id: Option<String>,
    /// Der Typ des Ereignisses, der zur Kategorisierung dient (z. B.
    /// "sensor.reading", "user.interaction").
    /// Hinweis: Wir verwenden ein echtes Feld `type` via raw identifier,
    /// damit Code und JSON-Name 1:1 übereinstimmen.
    pub r#type: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn aussen_event_roundtrip() {
        let mut features = BTreeMap::new();
        features.insert("temperature".to_string(), json!(22.5));

        let mut meta = BTreeMap::new();
        meta.insert("adapter".to_string(), json!("v1"));

        let event = AussenEvent {
            id: Some("uuid-123".to_string()),
            r#type: "sensor.reading".to_string(),
            source: "home-assistant".to_string(),
            title: Some("Living Room Temperature".to_string()),
            summary: Some("Temperature reading from the main sensor.".to_string()),
            url: Some("http://ha.local/sensor/123".to_string()),
            tags: Some(vec!["home".to_string(), "iot".to_string()]),
            ts: Some("2023-10-27T10:00:00Z".to_string()),
            features: Some(features),
            meta: Some(meta),
        };

        let serialized = serde_json::to_string(&event).expect("Serialization failed");

        // Ensure "r#type" is NOT in the JSON, but "type" IS.
        assert!(serialized.contains("\"type\":\"sensor.reading\""));
        assert!(!serialized.contains("r#type"));

        let deserialized: AussenEvent =
            serde_json::from_str(&serialized).expect("Deserialization failed");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn aussen_event_from_json_fixture() {
        let json_data = json!({
            "type": "link",
            "source": "test",
            "title": "Hello",
            "url": "https://example.org",
            "tags": ["demo"],
            "features": {}
        });

        let event: AussenEvent = serde_json::from_value(json_data).expect("Deserialization failed");
        assert_eq!(event.r#type, "link");
        assert_eq!(event.source, "test");
        assert_eq!(event.title, Some("Hello".to_string()));
        assert_eq!(event.url, Some("https://example.org".to_string()));
        assert_eq!(event.tags, Some(vec!["demo".to_string()]));
        assert!(event.features.is_some());
    }
}
