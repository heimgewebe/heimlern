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

/// Validates an event domain/namespace identifier.
///
/// This validates event namespace identifiers (e.g., "aussen", "sensor.v1"), not DNS domains.
/// Single-label identifiers like "aussen" are valid by design for internal event routing.
///
/// Rules (similar to DNS hostname rules but applied to namespace identifiers):
/// - Labels separated by dots, each 1-63 chars, total ≤253 chars
/// - Each label: starts/ends with alphanumeric, may contain hyphens in middle
/// - No whitespace, underscores, or leading/trailing dots
/// - No IDN/Unicode (ASCII alphanumeric + hyphens only)
///
/// Note: If future requirements need different characters (e.g., underscores, slashes),
/// this validation should be relaxed or the semantic meaning of "domain" clarified
/// with respect to the Chronik API contract.
pub fn is_valid_event_domain(domain: &str) -> bool {
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }

    let bytes = domain.as_bytes();
    // Structural fast fail: domain must not start or end with a dot.
    if bytes[0] == b'.' || bytes[bytes.len() - 1] == b'.' {
        return false;
    }

    // Note: any whitespace and non-ASCII bytes are rejected by the ASCII-only label checks below.
    for label in bytes.split(|&b| b == b'.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        // First and last bytes of each label must be ASCII alphanumeric.
        if !label[0].is_ascii_alphanumeric() || !label[label.len() - 1].is_ascii_alphanumeric() {
            return false;
        }
        // All bytes must be ASCII alphanumeric or hyphen.
        if !label
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn aussen_event_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
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
            url: Some("https://ha.local/sensor/123".to_string()),
            tags: Some(vec!["home".to_string(), "iot".to_string()]),
            ts: Some("2023-10-27T10:00:00Z".to_string()),
            features: Some(features),
            meta: Some(meta),
        };

        let serialized = serde_json::to_string(&event)?;

        // Ensure "r#type" is NOT in the JSON, but "type" IS (structural check).
        let v: serde_json::Value = serde_json::from_str(&serialized)?;
        assert_eq!(
            v.get("type").and_then(|x| x.as_str()),
            Some("sensor.reading")
        );
        assert!(v.get("r#type").is_none());

        let deserialized: AussenEvent = serde_json::from_str(&serialized)?;
        assert_eq!(event, deserialized);
        Ok(())
    }

    #[test]
    fn aussen_event_from_json_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let json_data = json!({
            "type": "link",
            "source": "test",
            "title": "Hello",
            "url": "https://example.org",
            "tags": ["demo"],
            "features": {}
        });

        let event: AussenEvent = serde_json::from_value(json_data)?;
        assert_eq!(event.r#type, "link");
        assert_eq!(event.source, "test");
        assert_eq!(event.title, Some("Hello".to_string()));
        assert_eq!(event.url, Some("https://example.org".to_string()));
        assert_eq!(event.tags, Some(vec!["demo".to_string()]));
        assert!(event.features.is_some());
        Ok(())
    }

    #[test]
    fn test_is_valid_event_domain() {
        assert!(is_valid_event_domain("example.com"));
        assert!(is_valid_event_domain("a.b.c"));
        assert!(is_valid_event_domain("my-domain.com"));
        assert!(is_valid_event_domain("x"));

        assert!(!is_valid_event_domain(""));
        assert!(!is_valid_event_domain(" "));
        assert!(!is_valid_event_domain(" example.com"));
        assert!(!is_valid_event_domain("example.com "));
        assert!(!is_valid_event_domain("ex ample.com"));
        assert!(!is_valid_event_domain(".start"));
        assert!(!is_valid_event_domain("end."));
        assert!(!is_valid_event_domain("my..domain"));
        assert!(!is_valid_event_domain("bad_char"));
        assert!(!is_valid_event_domain("-start"));
        assert!(!is_valid_event_domain("end-"));

        // Verify ASCII-only: Unicode characters should be rejected
        assert!(!is_valid_event_domain("café"));
        assert!(!is_valid_event_domain("日本"));
        assert!(!is_valid_event_domain("αβγ"));
        assert!(!is_valid_event_domain("domain.über"));
    }
}
