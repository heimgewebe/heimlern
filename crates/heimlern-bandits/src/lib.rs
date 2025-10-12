use heimlern_core::{Context,Decision,Policy};
use rand::prelude::*;
use rand::seq::SliceRandom;
use serde_json::json;

#[derive(Debug)]
pub struct RemindBandit {
    pub epsilon: f32,
    slots: Vec<String>,
}

impl Default for RemindBandit {
    fn default() -> Self {
        Self{ epsilon: 0.2, slots: vec!["morning".into(),"afternoon".into(),"evening".into()] }
    }
}

impl Policy for RemindBandit {
    fn decide(&mut self, ctx:&Context) -> Decision {
        let mut rng = thread_rng();
        let explore = rng.gen::<f32>() < self.epsilon;
        let action = if explore {
            self.slots.choose(&mut rng).unwrap().clone()
        } else {
            self.slots[0].clone() // TODO: spätere Werte-Schätzung
        };
        Decision{
            action: format!("remind.{}", action),
            score: 0.5,
            why: if explore { "explore ε" } else { "exploit heuristic" }.into(),
            context: Some(serde_json::to_value(ctx).unwrap())
        }
    }
    fn feedback(&mut self, _ctx:&Context, _action:&str, _reward:f32) {}
    fn snapshot(&self) -> serde_json::Value { json!({"epsilon": self.epsilon, "slots": self.slots}) }
    fn load(&mut self, v:serde_json::Value){
        if let Some(e)=v.get("epsilon").and_then(|x|x.as_f64()){ self.epsilon=e as f32; }
        if let Some(sl)=v.get("slots").and_then(|x|x.as_array()){
            self.slots=sl.iter().filter_map(|s| s.as_str().map(|x|x.to_string())).collect();
        }
    }
}
