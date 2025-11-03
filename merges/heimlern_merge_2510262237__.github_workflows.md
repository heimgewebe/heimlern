### 📄 .github/workflows/ci-rust.yml

**Größe:** 1 KB | **md5:** `0557d72bf7beeb0ad27e7edc112a8eb5`

```yaml
name: rust (cached)

on:
  push:
    paths:
      - "Cargo.toml"
      - "Cargo.lock"
      - "crates/**"
      - ".github/workflows/ci-rust.yml"
  pull_request:
    paths:
      - "Cargo.toml"
      - "Cargo.lock"
      - "crates/**"
      - ".github/workflows/ci-rust.yml"

permissions:
  contents: read

jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust (stable)
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache target
        uses: actions/cache@v4
        with:
          path: target
          key: ${{ runner.os }}-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy lint
        run: cargo clippy --workspace --all-targets --locked -- -D warnings

      - name: Build workspace
        run: cargo build --workspace --all-targets --locked

      - name: Test workspace
        run: cargo test --workspace --all-targets --locked --no-fail-fast
```

### 📄 .github/workflows/ci.yml

**Größe:** 455 B | **md5:** `789b20eee1f28d4998da06fd5df06b31`

```yaml
name: ci
on: [push, pull_request]
permissions:
  contents: read
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --workspace --all-targets --verbose
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: echo '{}' | cargo run -p heimlern-bandits --example decide
      - run: cargo test --workspace --all-targets --verbose
```

### 📄 .github/workflows/contracts.yml

**Größe:** 706 B | **md5:** `b2be86b2a1e0a2a3b057acca093fc5b5`

```yaml
name: contracts
permissions:
  contents: read
on:
  push:
  pull_request:
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.x'
      - run: python -m pip install --upgrade pip
      - run: pip install -r requirements-tools.txt
      - name: Generate examples
        run: python scripts/examples.py
      - name: Validate snapshot
        run: python scripts/validate_json.py contracts/policy_snapshot.schema.json /tmp/heimlern_snapshot.json
      - name: Validate feedback
        run: python scripts/validate_json.py contracts/policy_feedback.schema.json /tmp/heimlern_feedback.json
```

### 📄 .github/workflows/validate-aussen-samples.yml

**Größe:** 526 B | **md5:** `71d9760b4539abba75abb7a772ec3649`

```yaml
name: validate (aussen samples)
on: [push, pull_request, workflow_dispatch]

permissions:
  contents: read

jobs:
  validate:
    if: ${{ hashFiles('data/samples/aussensensor.jsonl') != '' }}
    uses: heimgewebe/metarepo/.github/workflows/reusable-validate-jsonl.yml@contracts-v1
    with:
      jsonl_paths_list: |
        data/samples/aussensensor.jsonl
      schema_url: https://raw.githubusercontent.com/heimgewebe/metarepo/contracts-v1/contracts/aussen.event.schema.json
      strict: false
      validate_formats: true
```

### 📄 .github/workflows/validate-aussen.yml

**Größe:** 1001 B | **md5:** `7180dd9eaa083d00a579f0d78af1fca1`

```yaml
name: validate (aussen in heimlern)
permissions:
  contents: read
on: [push, pull_request, workflow_dispatch]
jobs:
  samples:
    name: samples (data/samples/aussensensor.jsonl)
    if: ${{ hashFiles('data/samples/aussensensor.jsonl') != '' }}
    uses: heimgewebe/metarepo/.github/workflows/reusable-validate-jsonl.yml@contracts-v1
    with:
      jsonl_path: data/samples/aussensensor.jsonl
      schema_url: https://raw.githubusercontent.com/heimgewebe/metarepo/contracts-v1/contracts/aussen.event.schema.json
      strict: false
      validate_formats: true

  fixtures:
    name: fixtures (tests/fixtures/aussen.jsonl)
    if: ${{ hashFiles('tests/fixtures/aussen.jsonl') != '' }}
    uses: heimgewebe/metarepo/.github/workflows/reusable-validate-jsonl.yml@contracts-v1
    with:
      jsonl_path: tests/fixtures/aussen.jsonl
      schema_url: https://raw.githubusercontent.com/heimgewebe/metarepo/contracts-v1/contracts/aussen.event.schema.json
      strict: false
      validate_formats: true
```

