### 📄 crates/heimlern-core/examples/ingest_events.rs

**Größe:** 1 KB | **md5:** `d479419598cd714dfba65b2918710d5a`

```rust
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
        if event.title.as_ref().map(|t| !t.is_empty()).unwrap_or(false) {
            score += 0.3;
        }
        if let Some(tags) = &event.tags {
            score += (tags.len().min(5) as f32) * 0.04;
        }

        println!(
            "{score:.2}\t{}",
            event.title.as_deref().unwrap_or("<untitled>")
        );
    }

    Ok(())
}
```

