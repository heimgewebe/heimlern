use heimlern_core::{Context,Policy};
use heimlern_bandits::RemindBandit;

fn main(){
    let mut p = RemindBandit::default();
    let ctx = Context{ kind: "reminder".into(), features: serde_json::json!({"load": 0.3}) };
    let d = p.decide(&ctx);
    println!("{}", serde_json::to_string_pretty(&d).unwrap());
}
