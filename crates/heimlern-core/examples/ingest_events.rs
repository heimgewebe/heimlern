use heimlern_core::event::AussenEvent;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1);
    let reader: Box<dyn BufRead> = match path {
        Some(p) => Box::new(BufReader::new(File::open(p)?)),
        None => Box::new(BufReader::new(io::stdin())),
    };

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let event: AussenEvent = serde_json::from_str(&line)?;

        let mut score: f32 = 0.0;
        if event.url.is_some() {
            score += 0.5;
        }
        if event.title.as_ref().is_some_and(|t| !t.is_empty()) {
            score += 0.3;
        }
        if let Some(tags) = &event.tags {
            #[allow(clippy::cast_precision_loss)]
            let tag_score = (tags.len().min(5) as f32) * 0.04;
            score += tag_score;
        }

        println!(
            "{score:.2}\t{}",
            event.title.as_deref().unwrap_or("<untitled>")
        );
    }

    Ok(())
}
