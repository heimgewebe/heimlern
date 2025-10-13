//! Beispiel-Implementierung eines ε-greedy-Banditen für Erinnerungs-Slots.
//!
//! Der `RemindBandit` demonstriert, wie das [`Policy`](heimlern_core::Policy)-Trait
//! für das häusliche Erinnerungs-Szenario implementiert werden kann. Er wählt
//! mit Wahrscheinlichkeit `epsilon` zufällig einen Slot und fällt andernfalls
//! auf eine deterministische Heuristik zurück.

use heimlern_core::{Context, Decision, Policy};
use rand::prelude::*;
use rand::seq::SliceRandom;
use serde_json::json;

/// Einfache ε-greedy Policy für Erinnerungen.
#[derive(Debug)]
pub struct RemindBandit {
    /// Wahrscheinlichkeit für Explorationsschritte zwischen 0.0 und 1.0.
    pub epsilon: f32,
    slots: Vec<String>,
}

impl Default for RemindBandit {
    fn default() -> Self {
        Self {
            epsilon: 0.2,
            slots: vec!["morning".into(), "afternoon".into(), "evening".into()],
        }
    }
}

impl Policy for RemindBandit {
    /// Wählt einen Erinnerungs-Slot basierend auf ε-greedy Exploration.
    fn decide(&mut self, ctx: &Context) -> Decision {
        let mut rng = thread_rng();
        // Schutz: Falls Slots leer sind (z. B. nach fehlerhaftem Load), auf Defaults zurückfallen.
        if self.slots.is_empty() {
            self.slots = vec!["morning".into(), "afternoon".into(), "evening".into()];
        }
        let explore = rng.gen::<f32>() < self.epsilon;
        let action = if explore {
            // choose() ist jetzt safe, weil slots garantiert nicht leer ist.
            self.slots
                .choose(&mut rng)
                .expect("slots non-empty")
                .clone()
        } else {
            self.slots[0].clone() // TODO: spätere Werte-Schätzung
        };
        Decision {
            action: format!("remind.{}", action),
            score: 0.5,
            why: if explore {
                "explore ε"
            } else {
                "exploit heuristic"
            }
            .into(),
            context: Some(serde_json::to_value(ctx).unwrap()),
        }
    }
    /// Aktuell ungenutztes Feedback (Platzhalter für zukünftiges Lernen).
    fn feedback(&mut self, _ctx: &Context, _action: &str, _reward: f32) {}

    /// Persistiert `epsilon` und die bekannten Slots als JSON.
    fn snapshot(&self) -> serde_json::Value {
        json!({"epsilon": self.epsilon, "slots": self.slots})
    }

    /// Rekonstruiert `epsilon` und Slots aus einem Snapshot.
    fn load(&mut self, v: serde_json::Value) {
        if let Some(e) = v.get("epsilon").and_then(|x| x.as_f64()) {
            // clamp auf [0.0, 1.0]
            let e = e as f32;
            self.epsilon = if e.is_finite() {
                e.clamp(0.0, 1.0)
            } else {
                0.2
            };
        }
        if let Some(sl) = v.get("slots").and_then(|x| x.as_array()) {
            self.slots = sl
                .iter()
                .filter_map(|s| s.as_str().map(|x| x.to_string()))
                .collect();
            if self.slots.is_empty() {
                self.slots = vec!["morning".into(), "afternoon".into(), "evening".into()];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heimlern_core::{Context, Policy};
    use serde_json::json;

    #[test]
    fn snapshot_roundtrip_and_decide_prefix() {
        let mut agent = RemindBandit::default();
        agent.epsilon = 0.0; // deterministisch
        let snap = agent.snapshot();
        let mut restored = RemindBandit {
            epsilon: 1.0,
            slots: vec![],
        };
        restored.load(snap);
        assert!((restored.epsilon - 0.0).abs() < f32::EPSILON);
        assert!(!restored.slots.is_empty());
        let ctx = Context {
            kind: "reminder".into(),
            features: json!({"x":1}),
        };
        let d = restored.decide(&ctx);
        assert!(d.action.starts_with("remind."));
        assert!(!d.why.is_empty());
    }
}
