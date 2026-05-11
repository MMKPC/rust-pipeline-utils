# rust-pipeline-utils

High-performance CLI utilities for UE5/VFX asset pipeline workflows, written in Rust.

Provides BLAKE3 checksumming, parallel batch processing, USD dependency graph analysis, and USD layer stack resolution — all without requiring a DCC application.

## Commands

```
pipeline checksum <path> [--expected <hash>]
pipeline batch <input_dir> <output_dir> [--ext usda]
pipeline graph <path> [--json]
pipeline usd-resolve <path>
```

## Build

```bash
cargo build --release
./target/release/pipeline --help
```

## Test

```bash
cargo test
cargo clippy -- -D warnings
```

## Modules

| Module | Purpose |
|---|---|
| `checksum` | BLAKE3 file hashing and verification |
| `batch_processor` | Rayon parallel directory copy with extension filter |
| `dep_graph` | DFS cycle detection on `@./path@` USD references |
| `usd_resolver` | Recursive USD layer stack resolver (static, no USD runtime) |
| `config` | JSON-based pipeline configuration |
| `error` | Unified `PipelineError` type via `thiserror` |
