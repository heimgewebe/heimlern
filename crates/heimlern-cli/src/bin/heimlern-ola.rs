//! CLI wrapper for Rust-owned Operator Learning Axis normalization.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use heimlern_core::ola;
use serde_json::{json, Value};
use std::fs::File;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "Rust-owned OLA adapter normalization", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a redacted Grabowski friction record into OLA outcome JSON.
    Adapt {
        /// Input JSON file containing one redacted friction/outcome record.
        #[arg(long)]
        input: PathBuf,
        /// Output shape.
        #[arg(long, value_enum, default_value = "routing-outcome")]
        emit: Emit,
        /// Policy id to embed when emitting a decision outcome.
        #[arg(long, default_value = "grabowski-routing-v0")]
        policy_id: String,
    },
    /// Convert a routing outcome JSON record into decision-outcome JSON.
    DecisionOutcome {
        /// Input JSON file containing one routing outcome record.
        #[arg(long)]
        input: PathBuf,
        /// Policy id to embed in the decision outcome.
        #[arg(long, default_value = "grabowski-routing-v0")]
        policy_id: String,
    },
    /// Derive the safe policy delta key for a route action.
    RouteDeltaKey {
        /// Action string, e.g. route.direct:patch.
        action: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Emit {
    RoutingOutcome,
    DecisionOutcome,
}

fn read_json(path: &PathBuf) -> Result<Value> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    serde_json::from_reader(file).with_context(|| format!("failed to parse {}", path.display()))
}

fn print_json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Adapt {
            input,
            emit,
            policy_id,
        } => {
            let input_record = read_json(&input)?;
            let routing_outcome = ola::adapt(&input_record);
            let payload = if emit == Emit::DecisionOutcome {
                ola::to_decision_outcome(&routing_outcome, &policy_id)
            } else {
                routing_outcome
            };
            print_json(&payload)?;
        }
        Commands::DecisionOutcome { input, policy_id } => {
            let routing_outcome = read_json(&input)?;
            let payload = ola::to_decision_outcome(&routing_outcome, &policy_id);
            print_json(&payload)?;
        }
        Commands::RouteDeltaKey { action } => match ola::route_delta_key(&action) {
            Ok(route_key) => print_json(&json!({
                "delta_key": route_key.delta_key,
                "route": route_key.route
            }))?,
            Err(err) => {
                let payload = json!({
                    "kind": err.kind().as_str(),
                    "message": err.message()
                });
                eprintln!("{}", serde_json::to_string_pretty(&payload)?);
                std::process::exit(2);
            }
        },
    }
    Ok(())
}
