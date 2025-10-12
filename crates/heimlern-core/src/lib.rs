use serde::{Serialize,Deserialize};
use serde_json::Value;

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct Context { pub kind: String, pub features: Value }

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct Decision { pub action: String, pub score: f32, pub why: String, pub context: Option<Value> }

pub trait Policy {
    fn decide(&mut self, ctx: &Context) -> Decision;
    fn feedback(&mut self, ctx: &Context, action: &str, reward: f32);
    fn snapshot(&self) -> Value;
    fn load(&mut self, snapshot: Value);
}
