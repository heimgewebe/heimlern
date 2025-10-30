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
            if let Some((slot, _)) = self
                .slots
                .iter()
                // Ungültige Werte (NaN) ignorieren
                .filter_map(|s| {
                    let average = self.get_average_reward(s);
                    average.is_finite().then_some((s, average))
                })
                .max_by(|(_, a_avg), (_, b_avg)| a_avg.total_cmp(b_avg))
            {
                slot.clone()
            } else {
                // Falls alle Rewards NaN sind, trotzdem stabil zurückfallen
                eprintln!(
                    "[heimlern-bandits] decide(): alle Slots haben ungültige Rewards (NaN) – fallback"
                );
                return fallback_decision("invalid rewards", ctx);
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
        } else {
            // Klare Rückmeldung statt stillem Ignorieren.
            eprintln!(
                "[heimlern-bandits] feedback(): Aktion ohne erwartetes Präfix 'remind.': '{action}' – ignoriert"
            );
        }
    }

    /// Persistiert vollständigen Zustand als JSON.
    fn snapshot(&self) -> serde_json::Value {
        // Trait liefert Value, daher hier bewusst kein Result.
        // Fehler sind extrem unwahrscheinlich; im Fall der Fälle liefern wir Null
        // (explizit, ohne panic), damit Aufrufer deterministisch bleiben.
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Lädt Zustand aus Snapshot (robust mit Korrekturen).
    fn load(&mut self, v: serde_json::Value) {
        match serde_json::from_value::<RemindBandit>(v) {
            Ok(mut loaded) => {
                loaded.sanitize();
                *self = loaded;
            }
            Err(e) => {
                // Nicht schweigend schlucken: sichtbarer Hinweis auf STDOUT/ERR.
                eprintln!("[heimlern-bandits] load(): Snapshot konnte nicht geladen werden: {e}");
            }
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

    #[test]
    fn nan_rewards_are_ignored_in_exploit() {
        let mut bandit = RemindBandit {
            epsilon: 0.0, // Exploit only
            slots: vec!["a".into(), "b".into()],
            values: HashMap::new(),
        };
        let ctx = Context {
            kind: "t".into(),
            features: serde_json::json!({}),
        };
        bandit.feedback(&ctx, "remind.b", 0.5);
        bandit.values.insert("a".into(), (0, f32::NAN));

        let decision = bandit.decide(&ctx);
        assert!(decision.action.ends_with(".b"));
    }

    #[test]
    fn feedback_without_prefix_is_ignored_but_warns() {
        let mut bandit = RemindBandit::default();
        let ctx = Context {
            kind: "t".into(),
            features: serde_json::json!({}),
        };

        bandit.feedback(&ctx, "afternoon", 0.9);
        assert!(bandit.values.is_empty());
    }
}
