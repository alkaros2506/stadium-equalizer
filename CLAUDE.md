# CLAUDE.md

## Project Overview

Stadium Equalizer is a Rust workspace with 5 crates: `core` (DSP), `neural` (GRU model), `cli` (command-line), `web` (WASM bindings), and `tests` (integration tests). It also includes JS/TS wrapper packages under `packages/` for website integration.

## Build & Test Commands

```bash
# Build everything (native)
cargo build

# Run all tests (excludes web crate — it only compiles to wasm32)
cargo test --workspace --exclude stadium-eq-web

# Lint (CI enforces -Dwarnings)
RUSTFLAGS=-Dwarnings cargo clippy --workspace --exclude stadium-eq-web --all-targets

# Format check
cargo fmt --all -- --check

# Build WASM
cargo build -p stadium-eq-web --target wasm32-unknown-unknown --release

# Check no_std compatibility for core
cargo check -p stadium-eq-core --no-default-features

# Run JS/TS wrapper tests
cd packages/stadium-eq-js && npm test
cd packages/stadium-eq-react && npm test
```

## Key Architecture

- **Pipeline state machine**: Idle -> Calibrating -> Processing (or Bypassed). See `core/src/pipeline.rs`.
- **Signal chain**: STFT -> noise estimation -> VAD -> Wiener filter -> source separation -> mix gains -> spectral gate -> STFT synthesis -> biquad EQ -> limiter.
- **Two noise estimators**: SPP-MMSE (default) and Martin's Minimum Statistics, selected via `NoiseEstimatorType` in `core/src/config.rs`.
- **Neural model**: RNNoise-inspired architecture with 3 GRU layers producing 22 Bark-band gains. See `neural/src/model.rs`.
- **Web crate** is a `cdylib` with `extern "C"` functions — only compiles for `wasm32-unknown-unknown`. Exclude it from native clippy/test runs.

## Important Conventions

- **Always add tests** when introducing new features or modifying existing ones. Every new package, module, or significant behavior change must include corresponding test coverage.
- The `web` crate must always be excluded from `cargo test` and `cargo clippy` (it only targets wasm32).
- CI runs with `RUSTFLAGS=-Dwarnings` — all clippy warnings must be fixed before merging.
- The core crate supports `no_std` via the `std` feature flag (on by default). The web crate uses `default-features = false` for core.
- Linux builds require `libasound2-dev` for the `cpal` audio dependency in the CLI crate.
- JS/TS packages under `packages/` use Vitest for testing. Tests mock browser APIs (Web Audio, WASM, etc.) since they run in Node.

## Workspace Layout

```
core/       Core DSP: FFT, STFT, filters, noise estimation, VAD, pipeline
neural/     Neural network: GRU, feature extraction, band gain interpolation
cli/        CLI binary: file processing and real-time audio via cpal
web/        WASM cdylib: C-ABI bindings for browser use
tests/      Integration tests covering all major subsystems
web-ui/     Browser frontend (separate from Rust workspace)
packages/   JS/TS wrapper packages for website integration
  stadium-eq-js/      Vanilla JS/TS wrapper (StadiumEQ class)
  stadium-eq-react/   React hooks and components
```
