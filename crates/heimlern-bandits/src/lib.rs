use heimlern_core::{Context,Decision,Policy};
use rand::prelude::*;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct RemindBandit {
    pub epsilon: f32,
    slots: Vec<String>,
    values: HashMap<String, (u32, f32)>, // (pulls, total_reward)
}

impl Default for RemindBandit {
    fn default() -> Self {
        Self {
            epsilon: 0.2,
            slots: vec!["morning".into(), "afternoon".into(), "evening".into()],
            values: HashMap::new(),
        }
    }
}

impl Policy for RemindBandit {
    fn decide(&mut self, ctx: &Context) -> Decision {
        let mut rng = thread_rng();
        let explore = rng.gen::<f32>() < self.epsilon;

        if self.slots.is_empty() {
            return Decision {
                action: "remind.none".into(),
                score: 0.0,
                why: "no slots available".into(),
                context: Some(serde_json::to_value(ctx).unwrap()),
            };
        }

        let action = if explore {
            // Exploration: choose a random slot
            self.slots.choose(&mut rng).unwrap().clone()
        } else {
            // Exploitation: choose the best slot based on average reward.
            self.slots
                .iter()
                .max_by(|a, b| {
                    let val_a = self
                        .values
                        .get(*a)
                        .map(|(n, v)| if *n > 0 { v / *n as f32 } else { 0.0 })
                        .unwrap_or(0.0);
                    let val_b = self
                        .values
                        .get(*b)
                        .map(|(n, v)| if *n > 0 { v / *n as f32 } else { 0.0 })
                        .unwrap_or(0.0);
                    val_a
                        .partial_cmp(&val_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
                .clone()
        };

        let value_estimate = self
            .values
            .get(&action)
            .map(|(n, v)| if *n > 0 { v / *n as f32 } else { 0.0 })
            .unwrap_or(0.0);

        Decision {
            action: format!("remind.{}", action),
            score: value_estimate,
            why: if explore { "explore ε" } else { "exploit" }.into(),
            context: Some(serde_json::to_value(ctx).unwrap()),
        }
    }

    fn feedback(&mut self, _ctx: &Context, action: &str, reward: f32) {
        if let Some(slot) = action.strip_prefix("remind.") {
            let stats = self.values.entry(slot.to_string()).or_insert((0, 0.0));
            stats.0 += 1; // increment pulls
            stats.1 += reward; // add to total reward
        }
    }

    fn snapshot(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    fn load(&mut self, v: serde_json::Value) {
        if let Ok(loaded_bandit) = serde_json::from_value(v) {
            *self = loaded_bandit;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heimlern_core::Policy;

    #[test]
    fn test_bandit_learns_and_exploits() {
        let mut bandit = RemindBandit {
            epsilon: 0.0, // Disable exploration for deterministic testing
            slots: vec!["morning".into(), "afternoon".into(), "evening".into()],
            values: HashMap::new(),
        };
        let ctx = Context {
            kind: "test".into(),
            features: serde_json::Value::Null,
        };

        // Provide feedback: "afternoon" is the best slot.
        bandit.feedback(&ctx, "remind.morning", 0.1);
        bandit.feedback(&ctx, "remind.afternoon", 0.9);
        bandit.feedback(&ctx, "remind.evening", 0.3);
        bandit.feedback(&ctx, "remind.afternoon", 0.8);

        // In exploitation mode, the bandit should choose "afternoon".
        let decision = bandit.decide(&ctx);
        assert_eq!(decision.action, "remind.afternoon");
    }

    #[test]
    fn test_snapshot_and_load() {
        let mut bandit = RemindBandit {
            epsilon: 0.0,
            slots: vec!["a".into(), "b".into()],
            values: HashMap::new(),
        };
        let ctx = Context {
            kind: "test".into(),
            features: serde_json::Value::Null,
        };

        // Give feedback
        bandit.feedback(&ctx, "remind.b", 1.0);

        // Take a snapshot
        let snapshot = bandit.snapshot();

        // Create a new bandit and load the snapshot
        let mut new_bandit = RemindBandit::default();
        new_bandit.load(snapshot);

        // Verify the state has been loaded correctly
        assert_eq!(bandit.epsilon, new_bandit.epsilon);
        assert_eq!(bandit.slots, new_bandit.slots);
        assert_eq!(bandit.values, new_bandit.values);

        // Verify it makes the same decision
        let decision = new_bandit.decide(&ctx);
        assert_eq!(decision.action, "remind.b");
    }
}