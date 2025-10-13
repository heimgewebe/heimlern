+//! Beispiel-Implementierung eines ε-greedy-Banditen für Erinnerungs-Slots.
+//!
+//! Der `RemindBandit` demonstriert, wie das [`Policy`](heimlern_core::Policy)-Trait
+//! für das häusliche Erinnerungs-Szenario implementiert werden kann. Er wählt
+//! mit Wahrscheinlichkeit `epsilon` zufällig einen Slot (Exploration) und fällt
+//! andernfalls auf eine heuristische Auswertung der beobachteten Rewards zurück
+//! (Exploitation über durchschnittliche Belohnung pro Slot).
+
+use heimlern_core::{Context, Decision, Policy};
+use rand::prelude::*;
+use rand::seq::SliceRandom;
+use serde::{Deserialize, Serialize};
+use std::collections::HashMap;
+
+/// ε-greedy Policy für Erinnerungen.
+#[derive(Debug, Serialize, Deserialize)]
+pub struct RemindBandit {

* /// Wahrscheinlichkeit für Explorationsschritte zwischen 0.0 und 1.0.
* pub epsilon: f32,
* /// Verfügbare Zeit-Slots (Arme).
* pub slots: Vec<String>,
* /// Statistiken je Slot: (Anzahl Ziehungen, summierte Rewards).
* values: HashMap<String, (u32, f32)>,
  +}
*

+impl Default for RemindBandit {

* fn default() -> Self {
* ```
     Self {
  ```
* ```
         epsilon: 0.2,
  ```
* ```
         slots: vec!["morning".into(), "afternoon".into(), "evening".into()],
  ```
* ```
         values: HashMap::new(),
  ```
* ```
     }
  ```
* }
  +}
*

+impl Policy for RemindBandit {

* /// Wählt einen Erinnerungs-Slot basierend auf ε-greedy.
* fn decide(&mut self, ctx: &Context) -> Decision {
* ```
     let mut rng = thread_rng();
  ```
*
* ```
     // Fallback: Slots dürfen nie leer sein.
  ```
* ```
     if self.slots.is_empty() {
  ```
* ```
         self.slots = vec!["morning".into(), "afternoon".into(), "evening".into()];
  ```
* ```
     }
  ```
*
* ```
     let explore = rng.gen::<f32>() < self.epsilon;
  ```
*
* ```
     // Wenn aus irgendeinem Grund immer noch leer: sichere Rückgabe.
  ```
* ```
     if self.slots.is_empty() {
  ```
* ```
         return Decision {
  ```
* ```
             action: "remind.none".into(),
  ```
* ```
             score: 0.0,
  ```
* ```
             why: "no slots available".into(),
  ```
* ```
             context: Some(serde_json::to_value(ctx).unwrap()),
  ```
* ```
         };
  ```
* ```
     }
  ```
*
* ```
     let chosen_slot = if explore {
  ```
* ```
         // Exploration: zufällig wählen (safe, da nicht leer).
  ```
* ```
         self.slots.choose(&mut rng).unwrap().clone()
  ```
* ```
     } else {
  ```
* ```
         // Exploitation: Slot mit höchstem durchschnittlichem Reward.
  ```
* ```
         self.slots
  ```
* ```
             .iter()
  ```
* ```
             .max_by(|a, b| {
  ```
* ```
                 let val_a = self
  ```
* ```
                     .values
  ```
* ```
                     .get(*a)
  ```
* ```
                     .map(|(n, v)| if *n > 0 { v / *n as f32 } else { 0.0 })
  ```
* ```
                     .unwrap_or(0.0);
  ```
* ```
                 let val_b = self
  ```
* ```
                     .values
  ```
* ```
                     .get(*b)
  ```
* ```
                     .map(|(n, v)| if *n > 0 { v / *n as f32 } else { 0.0 })
  ```
* ```
                     .unwrap_or(0.0);
  ```
* ```
                 val_a
  ```
* ```
                     .partial_cmp(&val_b)
  ```
* ```
                     .unwrap_or(std::cmp::Ordering::Equal)
  ```
* ```
             })
  ```
* ```
             .unwrap()
  ```
* ```
             .clone()
  ```
* ```
     };
  ```
*
* ```
     let value_estimate = self
  ```
* ```
         .values
  ```
* ```
         .get(&chosen_slot)
  ```
* ```
         .map(|(n, v)| if *n > 0 { v / *n as f32 } else { 0.0 })
  ```
* ```
         .unwrap_or(0.0);
  ```
*
* ```
     Decision {
  ```
* ```
         action: format!("remind.{}", chosen_slot),
  ```
* ```
         score: value_estimate,
  ```
* ```
         why: if explore { "explore ε" } else { "exploit" }.into(),
  ```
* ```
         context: Some(serde_json::to_value(ctx).unwrap()),
  ```
* ```
     }
  ```
* }
*
* /// Nimmt Feedback entgegen und aktualisiert die Schätzung pro Slot.
* fn feedback(&mut self, _ctx: &Context, action: &str, reward: f32) {
* ```
     if let Some(slot) = action.strip_prefix("remind.") {
  ```
* ```
         let entry = self.values.entry(slot.to_string()).or_insert((0, 0.0));
  ```
* ```
         entry.0 += 1; // pulls
  ```
* ```
         entry.1 += reward; // total reward
  ```
* ```
     }
  ```
* }
*
* /// Persistiert vollständigen Zustand als JSON.
* fn snapshot(&self) -> serde_json::Value {
* ```
     serde_json::to_value(self).unwrap()
  ```
* }
*
* /// Lädt Zustand aus Snapshot (robust mit Korrekturen).
* fn load(&mut self, v: serde_json::Value) {
* ```
     if let Ok(mut loaded) = serde_json::from_value::<RemindBandit>(v) {
  ```
* ```
         // Korrigiere epsilon in den gültigen Bereich.
  ```
* ```
         if loaded.epsilon.is_finite() {
  ```
* ```
             loaded.epsilon = loaded.epsilon.clamp(0.0, 1.0);
  ```
* ```
         } else {
  ```
* ```
             loaded.epsilon = 0.2;
  ```
* ```
         }
  ```
* ```
         // Stelle sicher, dass Slots nicht leer sind.
  ```
* ```
         if loaded.slots.is_empty() {
  ```
* ```
             loaded.slots = vec!["morning".into(), "afternoon".into(), "evening".into()];
  ```
* ```
         }
  ```
* ```
         *self = loaded;
  ```
* ```
     }
  ```
* }
  +}
*

+#[cfg(test)]
+mod tests {

* use super::*;
* use heimlern_core::Policy;
*
* #[test]
* fn bandit_learns_and_exploits_best_slot() {
* ```
     let mut bandit = RemindBandit {
  ```
* ```
         epsilon: 0.0, // keine Exploration für deterministischen Test
  ```
* ```
         slots: vec!["morning".into(), "afternoon".into(), "evening".into()],
  ```
* ```
         values: HashMap::new(),
  ```
* ```
     };
  ```
* ```
     let ctx = Context {
  ```
* ```
         kind: "test".into(),
  ```
* ```
         features: serde_json::json!({"x":1}),
  ```
* ```
     };
  ```
*
* ```
     // Feedback: "afternoon" ist am besten.
  ```
* ```
     bandit.feedback(&ctx, "remind.morning", 0.1);
  ```
* ```
     bandit.feedback(&ctx, "remind.afternoon", 0.9);
  ```
* ```
     bandit.feedback(&ctx, "remind.evening", 0.3);
  ```
* ```
     bandit.feedback(&ctx, "remind.afternoon", 0.8);
  ```
*
* ```
     let decision = bandit.decide(&ctx);
  ```
* ```
     assert_eq!(decision.action, "remind.afternoon");
  ```
* ```
     assert!(decision.score > 0.5);
  ```
* }
*
* #[test]
* fn snapshot_roundtrip_retains_state() {
* ```
     let mut bandit = RemindBandit {
  ```
* ```
         epsilon: 0.33,
  ```
* ```
         slots: vec!["a".into(), "b".into()],
  ```
* ```
         values: HashMap::new(),
  ```
* ```
     };
  ```
* ```
     let ctx = Context {
  ```
* ```
         kind: "test".into(),
  ```
* ```
         features: serde_json::json!({"k":true}),
  ```
* ```
     };
  ```
* ```
     bandit.feedback(&ctx, "remind.b", 1.0);
  ```
*
* ```
     let snapshot = bandit.snapshot();
  ```
*
* ```
     let mut restored = RemindBandit::default();
  ```
* ```
     restored.load(snapshot);
  ```
*
* ```
     assert!((restored.epsilon - 0.33).abs() < f32::EPSILON);
  ```
* ```
     assert_eq!(restored.slots, vec!["a".to_string(), "b".to_string()]);
  ```
* ```
     assert_eq!(restored.values.get("b"), Some(&(1, 1.0)));
  ```
*
* ```
     // Gleiche Entscheidungserwartung nach Restore (mit epsilon 0.33 kann explo/explore schwanken,
  ```
* ```
     // aber der beste Slot bleibt b, wenn exploit gewählt wird).
  ```
* ```
     restored.epsilon = 0.0;
  ```
* ```
     let d = restored.decide(&ctx);
  ```
* ```
     assert_eq!(d.action, "remind.b");
  ```
* }
  +}
  EOF
  )