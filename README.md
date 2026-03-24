# Stadium Equalizer

Real-time audio equalizer and source separator for stadium and live-event environments. Calibrates to the acoustic environment, then lets you independently control crowd noise, speech, and music levels.

Built in Rust with support for native CLI, WebAssembly, and `no_std` embedded targets.

## Features

- **Environment calibration** -- automatically profiles noise characteristics and adapts processing parameters
- **Source separation** -- isolates crowd, speech, and music using spectral band energy analysis
- **Neural enhancement** -- RNNoise-inspired GRU network produces perceptual Bark-band gains
- **Wiener filtering** -- adaptive spectral noise suppression with configurable floor and oversubtraction
- **Voice activity detection** -- speech detection via energy, spectral flatness, and speech-band ratio
- **Real-time capable** -- lock-free ring buffers and frame-based processing for live audio
- **WebAssembly support** -- C-ABI WASM module for browser deployment
- **`no_std` core** -- the DSP core compiles without the standard library

## Project Structure

```
stadium-equalizer/
  core/       stadium-eq-core     DSP algorithms, pipeline, filters, FFT, STFT
  neural/     stadium-eq-neural   GRU model, feature extraction, band gains
  cli/        stadium-eq-cli      Command-line interface (file + real-time modes)
  web/        stadium-eq-web      WebAssembly C-ABI bindings
  tests/      stadium-eq-tests    Integration tests
  web-ui/                         Browser frontend
```

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) stable toolchain
- Linux: `sudo apt-get install libasound2-dev` (ALSA headers for `cpal`)
- WASM builds: `rustup target add wasm32-unknown-unknown`

### Build

```bash
cargo build --release
```

### Run the CLI

**Process a WAV file:**

```bash
cargo run --release -p stadium-eq-cli -- input.wav output.wav
```

**Real-time microphone processing:**

```bash
cargo run --release -p stadium-eq-cli
```

### CLI Options

```
USAGE: stadium-eq-cli [OPTIONS] [INPUT] [OUTPUT]

ARGS:
  [INPUT]                Input WAV file (omit for real-time mode)
  [OUTPUT]               Output WAV file (default: output.wav)

OPTIONS:
  --sample-rate <Hz>           Sample rate (default: 48000)
  --frame-size <N>             Samples per frame (default: 480)
  --fft-size <N>               FFT size (default: 1024)
  --calibrate-seconds <S>      Calibration duration (default: 7)
  --crowd <-1.0..1.0>          Crowd level: -1 suppress, +1 boost
  --speaker <-1.0..1.0>        Speaker level
  --music <-1.0..1.0>          Music level
  --gain <dB>                  Overall gain in dB
  --bypass                     Pass audio through unchanged
  --no-calibrate               Skip calibration phase
```

### Build for WebAssembly

```bash
rustup target add wasm32-unknown-unknown
cargo build -p stadium-eq-web --target wasm32-unknown-unknown --release
```

The `.wasm` binary is written to `target/wasm32-unknown-unknown/release/stadium_eq_web.wasm`.

## Architecture

### Processing Pipeline

```
Input -> STFT -> Noise Estimation -> VAD -> Band Energy Analysis
      -> Wiener Filter Gains -> Mix Controller Gains
      -> Gain Combination -> Temporal Smoothing -> Spectral Gate
      -> STFT Synthesis -> Biquad EQ -> Soft Limiter -> Output
```

### Pipeline States

| State | Behavior |
|---|---|
| **Idle** | Passthrough, waiting for calibration start |
| **Calibrating** | Collecting noise profile; input passed through |
| **Processing** | Full signal chain active |
| **Bypassed** | Manual passthrough mode |

### Calibration

Call `start_calibration()` to begin. The pipeline collects noise statistics for the configured duration (default 7 s), then automatically:

1. Classifies the environment (open stadium, dome arena, small venue, broadcast feed)
2. Estimates crowd density (sparse to roaring)
3. Measures dominant frequency range and SNR
4. Generates a tuned processing preset

### Mix Control

Source levels range from -1.0 (full suppression) to +1.0 (3x boost):

```rust
pipeline.set_mix(UserMix {
    crowd_level: -0.8,    // suppress crowd noise
    speaker_level: 0.5,   // boost speech
    music_level: 0.0,     // leave music unchanged
    overall_gain_db: 0.0,
});
```

## WASM API

The web crate exposes a C-ABI interface:

```c
Pipeline* stadium_eq_init(uint32_t sample_rate, uint32_t frame_size);
uint32_t  stadium_eq_process(Pipeline* ctx, const float* in, float* out, uint32_t len);
void      stadium_eq_start_calibration(Pipeline* ctx);
void      stadium_eq_set_mix(Pipeline* ctx, float crowd, float speaker, float music, float gain_db);
void      stadium_eq_set_bypass(Pipeline* ctx, uint32_t bypass);
void      stadium_eq_free(Pipeline* ctx);
float*    stadium_eq_alloc(uint32_t size);
void      stadium_eq_dealloc(float* ptr, uint32_t size);
```

## Development

### Run Tests

```bash
cargo test --workspace --exclude stadium-eq-web
```

### Lint

```bash
cargo clippy --workspace --exclude stadium-eq-web --all-targets
```

### Format

```bash
cargo fmt --all
```

### Check `no_std` Core

```bash
cargo check -p stadium-eq-core --no-default-features
```

## License

MIT
