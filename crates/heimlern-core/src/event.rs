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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    let domain = domain.trim();
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }
    if domain.contains(char::is_whitespace) {
        return false;
    }
    if domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }

    for label in domain.split('.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }

        // Each label must start and end with ASCII alphanumeric
        if !label.starts_with(|c: char| c.is_ascii_alphanumeric()) {
            return false;
        }
        if !label.ends_with(|c: char| c.is_ascii_alphanumeric()) {
            return false;
        }

        // All chars must be ASCII alphanumeric or hyphen
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_event_domain() {
        assert!(is_valid_event_domain("example.com"));
        assert!(is_valid_event_domain("a.b.c"));
        assert!(is_valid_event_domain("my-domain.com"));
        assert!(is_valid_event_domain("x"));

        assert!(!is_valid_event_domain(""));
        assert!(!is_valid_event_domain(" "));
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
