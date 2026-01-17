#![warn(clippy::unwrap_used, clippy::expect_used)]

//! Beispiel-Implementierung eines ε-greedy-Banditen für Erinnerungs-Slots.
//!
//! Der `RemindBandit` implementiert das [`Policy`]-Trait
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
#[inline]
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
    values: HashMap<String, (u32, f64)>,
}

// ---- Contract-Snapshot (gemäß contracts/policy.snapshot.schema.json) ----
#[derive(Debug, Serialize, Deserialize)]
struct ContractSnapshot {
    version: String,
    policy_id: String,
    ts: String,
    arms: Vec<String>,
    /// Anzahl der Feedbacks (Pulls) pro Arm.
    counts: Vec<u32>,
    /// Durchschnittlicher Reward pro Arm (Average Reward).
    /// ACHTUNG: Semantik ist "average", nicht "sum". Beim Laden muss
    /// `total = avg * count` berechnet werden.
    values: Vec<f64>,
    epsilon: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
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
    DEFAULT_SLOTS.iter().map(ToString::to_string).collect()
}

fn serialize_context(ctx: &Context) -> Option<serde_json::Value> {
    serde_json::to_value(ctx).ok()
}

fn fallback_decision(reason: &str, ctx: &Context) -> Decision {
    Decision {
        action: "remind.none".into(),
        score: 0.0,
        why: vec![reason.into()],
        context: serialize_context(ctx),
        chosen: None, // Wird ggf. vom Aufrufer gefüllt oder ist optional
    }
}

impl RemindBandit {
    /// Berechnet den durchschnittlichen Reward für einen Slot.
    fn get_average_reward(&self, slot: &str) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        {
            self.values.get(slot).map_or(0.0, |(n, v)| {
                if *n > 0 {
                    (v / f64::from(*n)) as f32
                } else {
                    0.0
                }
            })
        }
    }

    fn sanitize(&mut self) {
        if self.epsilon.is_finite() {
            self.epsilon = self.epsilon.clamp(0.0, 1.0);
        } else {
            self.epsilon = 0.0;
        }

        for (_, sum) in self.values.values_mut() {
            if !sum.is_finite() {
                *sum = 0.0;
            }
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
            why: vec![if explore { "explore ε" } else { "exploit" }.into()],
            context: serialize_context(ctx),
            chosen: None, // Optional, kann hier leer bleiben
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
            let slot_name = slot.to_string();
            if !self.slots.contains(&slot_name) {
                self.slots.push(slot_name.clone());
            }
            let entry = self.values.entry(slot_name).or_insert((0, 0.0));
            entry.0 += 1; // pulls
            entry.1 += f64::from(reward); // total reward
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
            let epsilon = if snap.epsilon.is_finite() {
                snap.epsilon.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let arms_empty = snap.arms.is_empty();
            if arms_empty {
                log_warn("load(): Snapshot ohne arms ist ungültig – verworfen");
                return;
            }

            let counts_len = snap.counts.len();
            let values_len = snap.values.len();
            let arms = snap.arms;
            let expected_len = arms.len();
            // counts/values müssen zur Länge der Arme passen, sonst ist der Snapshot ungültig.
            let lengths_match = counts_len == expected_len && values_len == expected_len;
            if !lengths_match {
                log_warn(&format!(
                    "load(): counts/values-Länge passt nicht zu arms (arms={expected_len}, counts={counts_len}, values={values_len})"
                ));
                return;
            }

            let counts = snap.counts;
            let values = snap.values;

            // Rückbau avg → totals: total = avg * n
            let mut map = HashMap::new();
            for (arm, (n, avg)) in arms.iter().zip(counts.iter().zip(values.iter())) {
                #[allow(clippy::cast_precision_loss)]
                let total = if *n > 0 && avg.is_finite() {
                    avg * f64::from(*n)
                } else {
                    0.0
                };
                map.insert(arm.clone(), (*n, total));
            }
            self.epsilon = epsilon;
            self.slots = arms;
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
                log_warn(&format!(
                    "load(): Snapshot konnte nicht geladen werden: {e}"
                ));
            }
        }
    }
}

// ---- kleine Helfer ----
fn iso8601_now() -> String {
    // RFC3339/ISO-8601-konformer UTC-Zeitstempel, z. B. "2025-11-09T12:34:56Z"
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

// ---- Contract-konforme Snapshot/Load-Implementierung (ersetzt Dummy oben) ----
impl RemindBandit {
    /// Persistiert Zustand als Contract-Snapshot (JSON-konform zum Schema).
    #[must_use]
    pub fn to_contract_snapshot(&self) -> serde_json::Value {
        let epsilon = if self.epsilon.is_finite() {
            self.epsilon.clamp(0.0, 1.0)
        } else {
            0.0
        };

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
            let sanitized_sum = if sum.is_finite() { sum } else { 0.0 };
            counts.push(n);
            #[allow(clippy::cast_precision_loss)]
            let avg = if n > 0 {
                sanitized_sum / f64::from(n)
            } else {
                0.0
            };
            values.push(avg);
        }
        let snap = ContractSnapshot {
            version: "0.1.0".into(),
            policy_id: "remind-bandit".into(),
            ts: iso8601_now(),
            arms,
            counts,
            values,
            epsilon,
            seed: None,
        };

        serde_json::to_value(snap).unwrap_or_else(|e| {
            log_warn(&format!(
                "to_contract_snapshot(): Snapshot konnte nicht serialisiert werden: {e}"
            ));
            serde_json::Value::Null
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heimlern_core::Policy;
    use serde_json::Value;

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
    fn snapshot_sanitizes_non_finite_values() {
        let mut bandit = RemindBandit {
            epsilon: f32::NAN,
            slots: vec!["a".into()],
            values: HashMap::new(),
        };
        bandit.values.insert("a".into(), (1, f64::INFINITY));

        let snapshot = bandit.snapshot();

        assert!(snapshot.is_object(), "Snapshot darf nicht Null werden");

        let Some(epsilon) = snapshot.get("epsilon").and_then(Value::as_f64) else {
            panic!("epsilon muss vorhanden sein")
        };
        assert_eq!(epsilon, 0.0);

        let Some(values) = snapshot.get("values").and_then(Value::as_array) else {
            panic!("values müssen eine Liste sein")
        };
        assert_eq!(values[0].as_f64(), Some(0.0));
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
    fn sanitize_resets_non_finite_sums_and_slots() {
        let mut bandit = RemindBandit {
            epsilon: 0.5,
            slots: vec![],
            values: HashMap::new(),
        };
        bandit.values.insert("a".into(), (2, f64::NAN));
        bandit.values.insert("b".into(), (3, f64::INFINITY));

        bandit.sanitize();

        assert_eq!(bandit.slots, default_slots());
        assert_eq!(bandit.values.get("a"), Some(&(2, 0.0)));
        assert_eq!(bandit.values.get("b"), Some(&(3, 0.0)));
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
        bandit.values.insert("a".into(), (0, f64::NAN));

        let decision = bandit.decide(&ctx);
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        {
            assert!(decision.action.ends_with(".b"));
        }
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
        #[allow(clippy::float_cmp)]
        {
            assert_eq!(decision.score, 0.0);
        }
    }

    #[test]
    fn feedback_adds_unknown_slot_and_prefers_it_on_exploit() {
        let mut bandit = RemindBandit {
            epsilon: 0.0, // exploit only for determinism
            slots: vec!["morning".into(), "evening".into()],
            values: HashMap::new(),
        };
        let ctx = Context {
            kind: "t".into(),
            features: serde_json::json!({}),
        };

        // Provide feedback for a slot not yet known to the bandit.
        bandit.feedback(&ctx, "remind.night", 1.0);

        // The slot should be added and chosen when exploiting.
        let decision = bandit.decide(&ctx);
        assert_eq!(decision.action, "remind.night");
        assert!(bandit.slots.contains(&"night".to_string()));
    }

    #[test]
    fn contract_snapshot_roundtrip_structure() {
        let mut bandit = RemindBandit {
            epsilon: 0.4,
            slots: vec!["m".into(), "a".into()],
            values: HashMap::new(),
        };
        let ctx = Context {
            kind: "t".into(),
            features: serde_json::json!({}),
        };
        bandit.feedback(&ctx, "remind.m", 1.0);
        bandit.feedback(&ctx, "remind.m", 0.0);
        bandit.feedback(&ctx, "remind.a", 0.5);

        let snap = bandit.snapshot();
        // Erwartete Felder laut Schema:
        for key in &[
            "version",
            "policy_id",
            "ts",
            "arms",
            "counts",
            "values",
            "epsilon",
        ] {
            assert!(snap.get(key).is_some(), "missing key {key}");
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
        let ctx = Context {
            kind: "t".into(),
            features: serde_json::json!({}),
        };
        // x: drei Feedbacks (Summe 1.2) -> n=3, avg=0.4
        bandit.feedback(&ctx, "remind.x", 0.2);
        bandit.feedback(&ctx, "remind.x", 0.5);
        bandit.feedback(&ctx, "remind.x", 0.5);
        // y: zwei Feedbacks (Summe 0.0) -> n=2, avg=0.0
        bandit.feedback(&ctx, "remind.y", 0.0);
        bandit.feedback(&ctx, "remind.y", 0.0);
        // z: kein Feedback -> n=0, avg=0.0

        let snap = bandit.to_contract_snapshot();
        let Some(arms) = snap["arms"].as_array() else {
            panic!("Feld 'arms' ist kein Array")
        };
        let Some(counts) = snap["counts"].as_array() else {
            panic!("Feld 'counts' ist kein Array")
        };
        let Some(values) = snap["values"].as_array() else {
            panic!("Feld 'values' ist kein Array")
        };
        assert_eq!(
            arms,
            &vec!["x", "y", "z"]
                .into_iter()
                .map(|s| serde_json::Value::String(s.into()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            counts,
            &vec![3, 2, 0]
                .into_iter()
                .map(serde_json::Value::from)
                .collect::<Vec<_>>()
        );
        // floats: 0.4, 0.0, 0.0
        let Some(val1) = values[0].as_f64() else {
            panic!("Wert ist nicht als f64 lesbar")
        };
        assert!((val1 - 0.4).abs() < 1e-6);

        let Some(val2) = values[1].as_f64() else {
            panic!("Wert ist nicht als f64 lesbar")
        };
        assert!((val2 - 0.0).abs() < 1e-6);

        let Some(val3) = values[2].as_f64() else {
            panic!("Wert ist nicht als f64 lesbar")
        };
        assert!((val3 - 0.0).abs() < 1e-6);
    }

    #[test]
    fn load_rejects_snapshot_with_wrong_policy_id() {
        // 1. Setup a bandit with NON-DEFAULT state (epsilon = 0.99)
        let source_bandit = RemindBandit {
            epsilon: 0.99,
            ..Default::default()
        };

        let mut snapshot_json = source_bandit.snapshot();

        // 2. Mangle the policy_id
        snapshot_json["policy_id"] = serde_json::Value::String("wrong-policy".into());

        // 3. Create a target bandit with DIFFERENT state (default epsilon = 0.2)
        let mut target_bandit = RemindBandit::default();
        let original_target_epsilon = target_bandit.epsilon;
        assert!((original_target_epsilon - 0.2).abs() < f32::EPSILON);

        // 4. Load the mangled snapshot
        target_bandit.load(snapshot_json);

        // 5. Verify that the state was NOT updated (should still be 0.2, not 0.99)
        #[allow(clippy::float_cmp)]
        {
            assert_eq!(target_bandit.epsilon, original_target_epsilon);
        }
    }

    #[test]
    fn load_rejects_snapshot_with_empty_arms() {
        // Snapshot mit leerem arms-Array darf den Zustand nicht überschreiben.
        let mut bandit = RemindBandit {
            epsilon: 0.77,
            slots: vec!["x".into()],
            values: HashMap::from([("x".into(), (1, 0.5))]),
        };

        let invalid_snapshot = serde_json::json!({
            "version": "0.1.0",
            "policy_id": "remind-bandit",
            "ts": "2024-01-01T00:00:00Z",
            "arms": [],
            "counts": [],
            "values": [],
            "epsilon": 0.1
        });

        bandit.load(invalid_snapshot);

        #[allow(clippy::float_cmp)]
        {
            assert_eq!(bandit.epsilon, 0.77);
        }
        assert_eq!(bandit.slots, vec!["x".to_string()]);
        assert_eq!(bandit.values.get("x"), Some(&(1, 0.5)));
    }

    #[test]
    fn load_rejects_snapshot_with_mismatching_lengths() {
        let mut bandit = RemindBandit {
            epsilon: 0.55,
            slots: vec!["a".into(), "b".into()],
            values: HashMap::from([("a".into(), (2, 1.0)), ("b".into(), (1, 0.2))]),
        };

        // counts und values haben unterschiedliche Längen -> Snapshot muss verworfen werden.
        let invalid_snapshot = serde_json::json!({
            "version": "0.1.0",
            "policy_id": "remind-bandit",
            "ts": "2024-01-01T00:00:00Z",
            "arms": ["a", "b"],
            "counts": [5],
            "values": [0.1, 0.2],
            "epsilon": 0.1
        });

        bandit.load(invalid_snapshot);

        #[allow(clippy::float_cmp)]
        {
            assert_eq!(bandit.epsilon, 0.55);
        }
        assert_eq!(bandit.slots, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(bandit.values.get("a"), Some(&(2, 1.0)));
        assert_eq!(bandit.values.get("b"), Some(&(1, 0.2)));
    }

    #[test]
    fn snapshot_maintains_f64_precision() {
        let mut bandit = RemindBandit {
            epsilon: 0.1,
            slots: vec!["high_precision".into()],
            values: HashMap::new(),
        };
        // Ein Wert mit vielen Dezimalstellen, der in f32 nicht exakt darstellbar ist.
        // 123456.789012345 hat 15 signifikante Stellen (f64 kann ~15-17, f32 nur ~7).
        let precise_val: f64 = 123_456.789_012_345;
        let count = 1;
        bandit
            .values
            .insert("high_precision".into(), (count, precise_val));

        let snap = bandit.snapshot();

        // Roundtrip
        let mut restored = RemindBandit::default();
        restored.load(snap);

        let Some((_, restored_sum)) = restored.values.get("high_precision") else {
            panic!("slot missing");
        };

        // 1. Absolute Genauigkeit: Muss in f64-Nähe sein (sehr kleine Toleranz).
        let diff = (restored_sum - precise_val).abs();
        assert!(diff < 1e-9, "f64-Präzision ging verloren: diff={diff:.15}");

        // 2. Vergleich gegen f32:
        // Beweist, dass wir tatsächlich besser sind als eine f32-Speicherung gewesen wäre.
        #[allow(clippy::cast_possible_truncation)]
        let f32_representation = precise_val as f32;
        let f32_loss = (f64::from(f32_representation) - precise_val).abs();

        assert!(
            diff < f32_loss,
            "Snapshot ist nicht präziser als f32! diff={diff:.15}, f32_loss={f32_loss:.15}"
        );
    }

    #[test]
    fn snapshot_large_count_reconstruction_precision() {
        // Testet den Pfad: total (start) -> avg (snapshot) -> total (load)
        // mit großen Zahlen und Divisionen, um Rundungsfehler zu provozieren.
        let mut bandit = RemindBandit {
            epsilon: 0.1,
            slots: vec!["heavy_usage".into()],
            values: HashMap::new(),
        };

        // 30 Mio Pulls. Total enthält Nachkommastellen, die bei 10^7 in f32 nicht darstellbar sind.
        // Bei 10^7 ist die Schrittweite von f32 bereits 1.0, d.h. .125 würde abgeschnitten.
        let count = 30_000_000;
        let total_reward: f64 = 10_000_000.125;
        bandit
            .values
            .insert("heavy_usage".into(), (count, total_reward));

        let snap = bandit.snapshot();

        // Roundtrip
        let mut restored = RemindBandit::default();
        restored.load(snap);

        let Some((_, restored_total)) = restored.values.get("heavy_usage") else {
            panic!("slot missing");
        };

        // 1. Check f64 precision
        let diff = (restored_total - total_reward).abs();
        // Erwarte extrem kleine Abweichung (f64 precision bei 10^7 ist ca 1e-9)
        assert!(diff < 1e-8, "Reconstruction error too high for f64: {diff}");

        // 2. Compare with f32 hypothetical loss
        // Wenn wir das in f32 gemacht hätten:
        #[allow(clippy::cast_possible_truncation)]
        let f32_total = total_reward as f32;
        #[allow(clippy::cast_possible_truncation)]
        let f32_count = count as f32;
        let f32_avg = f32_total / f32_count;
        let f32_reconstructed = f32_avg * f32_count;
        let f32_loss = (f64::from(f32_reconstructed) - total_reward).abs();

        // Der f64-Fehler muss signifikant kleiner sein als der f32-Fehler.
        assert!(
            diff < f32_loss,
            "f64 roundtrip ({diff}) not better than f32 ({f32_loss})"
        );
    }
}
