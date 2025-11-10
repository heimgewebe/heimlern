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
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// Logging-Helfer:
/// Mit Feature `telemetry` → `tracing::warn!`, sonst → `eprintln!`.
#[inline(always)]
fn log_warn(msg: &str) {
    #[cfg(feature = "telemetry")]
    {
        use tracing::warn;
        warn!(target: "heimlern-bandits", "{msg}");
    }
    #[cfg(not(feature = "telemetry"))]
    {
        eprintln!("[heimlern-bandits] {msg}");
    }
}

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

// ---- Contract-Snapshot (gemäß contracts/policy_snapshot.schema.json) ----
#[derive(Debug, Serialize, Deserialize)]
struct ContractSnapshot {
    version: String,
    policy_id: String,
    ts: String,
    arms: Vec<String>,
    counts: Vec<u32>,
    values: Vec<f32>,
    epsilon: f32,
    #[serde(skip_serializing_if = "Option::is_none")] seed: Option<u64>,
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
                log_warn("decide(): alle Slots haben ungültige Rewards (NaN) – fallback");
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
        if !reward.is_finite() {
            log_warn(&format!(
                "feedback(): ungültiger Reward '{reward}' für Aktion '{action}' – ignoriert"
            ));
            return;
        }
        if let Some(slot) = action.strip_prefix("remind.") {
            let entry = self.values.entry(slot.to_string()).or_insert((0, 0.0));
            entry.0 += 1; // pulls
            entry.1 += reward; // total reward
        } else {
            // Klare Rückmeldung statt stillem Ignorieren.
            log_warn(&format!(
                "feedback(): Aktion ohne erwartetes Präfix 'remind.': '{action}' – ignoriert"
            ));
        }
    }

    /// Persistiert Zustand als Contract-Snapshot (JSON-konform zum Schema).
    fn snapshot(&self) -> serde_json::Value {
        self.to_contract_snapshot()
    }

    /// Lädt Zustand aus einem Contract-Snapshot (robust, mit Sanitisierung).
    fn load(&mut self, v: serde_json::Value) {
        // Unterstütze sowohl altes („direct self“) als auch neues Contract-Format:
        // 1) Versuch: ContractSnapshot
        if let Ok(snap) = serde_json::from_value::<ContractSnapshot>(v.clone()) {
            if snap.policy_id != "remind-bandit" {
                log_warn(&format!(
                    "load(): falsche policy_id '{}' im Snapshot, erwarte 'remind-bandit'.",
                    snap.policy_id
                ));
                return; // Nicht laden.
            }
            self.epsilon = if snap.epsilon.is_finite() {
                snap.epsilon.clamp(0.0, 1.0)
            } else {
                0.0
            };
            self.slots = if snap.arms.is_empty() {
                default_slots()
            } else {
                snap.arms
            };
            // Rückbau avg → totals: total = avg * n
            let mut map = HashMap::new();
            let len = self.slots.len();
            for i in 0..len {
                let n = snap.counts.get(i).copied().unwrap_or(0);
                let avg = snap.values.get(i).copied().unwrap_or(0.0);
                let total = if n > 0 && avg.is_finite() { avg * n as f32 } else { 0.0 };
                map.insert(self.slots[i].clone(), (n, total));
            }
            self.values = map;
            self.sanitize();
            return;
        }
        // 2) Fallback: alte Form (direkte Struct-Serialization)
        match serde_json::from_value::<RemindBandit>(v) {
            Ok(mut legacy) => {
                legacy.sanitize();
                *self = legacy;
            }
            Err(e) => {
                // Nicht schweigend schlucken: sichtbarer Hinweis für Betreiber:innen.
                log_warn(&format!("load(): Snapshot konnte nicht geladen werden: {e}"));
            }
        }
    }
}

// ---- kleine Helfer ----
fn to_value_or_null<T: Serialize>(t: T) -> serde_json::Value {
    serde_json::to_value(t).unwrap_or(serde_json::Value::Null)
}
fn iso8601_now() -> String {
    // RFC3339/ISO-8601-konformer UTC-Zeitstempel, z. B. "2025-11-09T12:34:56Z"
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

// ---- Contract-konforme Snapshot/Load-Implementierung (ersetzt Dummy oben) ----
impl RemindBandit {
    /// Persistiert Zustand als Contract-Snapshot (JSON-konform zum Schema).
    pub fn to_contract_snapshot(&self) -> serde_json::Value {
        // Slots in stabiler Reihenfolge exportieren:
        let mut arms = self.slots.clone();
        if arms.is_empty() {
            arms = default_slots();
        }
        // Für jeden Arm counts/avg-Werte bereitstellen:
        let mut counts = Vec::with_capacity(arms.len());
        let mut values = Vec::with_capacity(arms.len());
        for arm in &arms {
            let (n, sum) = self.values.get(arm).copied().unwrap_or((0, 0.0));
            counts.push(n);
            let avg = if n > 0 { sum / n as f32 } else { 0.0 };
            values.push(avg);
        }
        let snap = ContractSnapshot {
            version: "0.1.0".into(),
            policy_id: "remind-bandit".into(),
            ts: iso8601_now(),
            arms,
            counts,
            values,
            epsilon: self.epsilon.clamp(0.0, 1.0),
            seed: None,
        };
        to_value_or_null(snap)
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

    #[test]
    fn feedback_with_nan_reward_is_ignored() {
        let mut bandit = RemindBandit {
            epsilon: 0.0,
            slots: vec!["a".into()],
            values: HashMap::new(),
        };
        let ctx = Context {
            kind: "t".into(),
            features: serde_json::json!({}),
        };

        bandit.feedback(&ctx, "remind.a", f32::NAN);
        let decision = bandit.decide(&ctx);

        assert_eq!(decision.action, "remind.a");
        assert_eq!(decision.score, 0.0);
    }

    #[test]
    fn contract_snapshot_roundtrip_structure() {
        let mut bandit = RemindBandit {
            epsilon: 0.4,
            slots: vec!["m".into(), "a".into()],
            values: HashMap::new(),
        };
        let ctx = Context { kind: "t".into(), features: serde_json::json!({}) };
        bandit.feedback(&ctx, "remind.m", 1.0);
        bandit.feedback(&ctx, "remind.m", 0.0);
        bandit.feedback(&ctx, "remind.a", 0.5);

        let snap = bandit.snapshot();
        // Erwartete Felder laut Schema:
        for key in &["version","policy_id","ts","arms","counts","values","epsilon"] {
            assert!(snap.get(key).is_some(), "missing key {}", key);
        }
        // Längen müssen passen
        match snap["arms"].as_array() {
            Some(array) => assert_eq!(array.len(), 2),
            None => panic!("Feld 'arms' ist kein Array"),
        }
        match snap["counts"].as_array() {
            Some(array) => assert_eq!(array.len(), 2),
            None => panic!("Feld 'counts' ist kein Array"),
        }
        match snap["values"].as_array() {
            Some(array) => assert_eq!(array.len(), 2),
            None => panic!("Feld 'values' ist kein Array"),
        }

        // Load zurück
        let mut restored = RemindBandit::default();
        restored.load(snap);
        // epsilon geklemmt + slots übernommen
        assert!((restored.epsilon - 0.4).abs() < f32::EPSILON);
        assert_eq!(restored.slots, vec!["m".to_string(), "a".to_string()]);
        // Exploit-only sollte stabil den höheren Durchschnitt ziehen (m: 0.5 vs a: 0.5 -> stabil erlaubt beide; wir prüfen nur Nicht-Panik)
        restored.epsilon = 0.0;
        let _ = restored.decide(&ctx);
    }

    #[test]
    fn contract_snapshot_semantics_counts_values() {
        let mut bandit = RemindBandit {
            epsilon: 0.3,
            slots: vec!["x".into(), "y".into(), "z".into()],
            values: HashMap::new(),
        };
        let ctx = Context { kind: "t".into(), features: serde_json::json!({}) };
        // x: drei Feedbacks (Summe 1.2) -> n=3, avg=0.4
        bandit.feedback(&ctx, "remind.x", 0.2);
        bandit.feedback(&ctx, "remind.x", 0.5);
        bandit.feedback(&ctx, "remind.x", 0.5);
        // y: zwei Feedbacks (Summe 0.0) -> n=2, avg=0.0
        bandit.feedback(&ctx, "remind.y", 0.0);
        bandit.feedback(&ctx, "remind.y", 0.0);
        // z: kein Feedback -> n=0, avg=0.0

        let snap = bandit.to_contract_snapshot();
        let arms = match snap["arms"].as_array() {
            Some(array) => array,
            None => panic!("Feld 'arms' ist kein Array"),
        };
        let counts = match snap["counts"].as_array() {
            Some(array) => array,
            None => panic!("Feld 'counts' ist kein Array"),
        };
        let values = match snap["values"].as_array() {
            Some(array) => array,
            None => panic!("Feld 'values' ist kein Array"),
        };
        assert_eq!(arms,   &vec!["x","y","z"].into_iter().map(|s| serde_json::Value::String(s.into())).collect::<Vec<_>>());
        assert_eq!(counts, &vec![3,2,0].into_iter().map(serde_json::Value::from).collect::<Vec<_>>());
        // floats: 0.4, 0.0, 0.0
        let val1 = match values[0].as_f64() {
            Some(v) => v,
            None => panic!("Wert ist nicht als f64 lesbar"),
        };
        assert!((val1 - 0.4).abs() < 1e-6);

        let val2 = match values[1].as_f64() {
            Some(v) => v,
            None => panic!("Wert ist nicht als f64 lesbar"),
        };
        assert!((val2 - 0.0).abs() < 1e-6);

        let val3 = match values[2].as_f64() {
            Some(v) => v,
            None => panic!("Wert ist nicht als f64 lesbar"),
        };
        assert!((val3 - 0.0).abs() < 1e-6);
    }

    #[test]
    fn load_rejects_snapshot_with_wrong_policy_id() {
        let mut bandit = RemindBandit::default();
        let original_epsilon = bandit.epsilon;

        let mut snapshot_json = bandit.snapshot();
        snapshot_json["policy_id"] = serde_json::Value::String("wrong-policy".into());

        bandit.load(snapshot_json);

        // Verify that the bandit's state has not changed
        assert_eq!(bandit.epsilon, original_epsilon);
    }
}
