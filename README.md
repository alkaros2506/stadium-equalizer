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

## Usage

### JavaScript / TypeScript (Browser)

The easiest way to use Stadium Equalizer in a web project is via the `stadium-eq` package, which wraps the WASM module behind a Web Audio API worklet.

```bash
npm install stadium-eq
```

```js
import { StadiumEQ } from "stadium-eq";

const eq = new StadiumEQ({ wasmUrl: "/stadium_eq.wasm" });
await eq.start();          // requests microphone, boots WASM pipeline
eq.calibrate();            // profile the environment (~7 s)
eq.setMix({
  crowd: -0.8,             // suppress crowd noise
  speaker: 0.5,            // boost speech
  music: 0.0,              // leave music unchanged
  gainDb: 0,
});

// later…
eq.stop();
```

You can also pass your own audio source instead of the default microphone:

```js
const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
const eq = new StadiumEQ({ wasmUrl: "/stadium_eq.wasm", audioSource: stream });
```

#### Events

```js
eq.on("statuschange", (status) => console.log(status));
// "idle" | "loading" | "calibrating" | "processing" | "bypassed" | "error"

eq.on("error", (msg) => console.error(msg));
eq.on("ready", () => console.log("Pipeline running"));
```

### React

The `stadium-eq-react` package provides a hook and a pre-built component.

```bash
npm install stadium-eq stadium-eq-react
```

**Hook:**

```tsx
import { useStadiumEQ } from "stadium-eq-react";

function App() {
  const eq = useStadiumEQ({ wasmUrl: "/stadium_eq.wasm" });

  return (
    <div>
      <p>Status: {eq.status}</p>
      <button onClick={eq.isRunning ? eq.stop : eq.start}>
        {eq.isRunning ? "Stop" : "Start"}
      </button>
      <button onClick={eq.calibrate} disabled={!eq.isRunning}>
        Calibrate
      </button>
      <input
        type="range" min="-1" max="1" step="0.01"
        value={eq.mix.crowd}
        onChange={e => eq.setMix({ crowd: Number(e.target.value) })}
      />
    </div>
  );
}
```

**Drop-in component** (includes controls + spectrum visualizer):

```tsx
import { StadiumEqualizer } from "stadium-eq-react";

function App() {
  return <StadiumEqualizer wasmUrl="/stadium_eq.wasm" />;
}
```

### Rust Library

Add the core crate to your `Cargo.toml`:

```toml
[dependencies]
stadium-eq-core = { git = "https://github.com/alkaros2506/stadium-equalizer", version = "0.1.0" }
```

```rust
use stadium_eq_core::{Pipeline, PipelineConfig, UserMix};

let config = PipelineConfig::default();   // 48 kHz, 480-sample frames
let mut pipeline = Pipeline::new(config);

pipeline.start_calibration();
// feed ~7 s of audio frames…

pipeline.set_mix(UserMix {
    crowd_level: -0.8,
    speaker_level: 0.5,
    music_level: 0.0,
    overall_gain_db: 0.0,
});

let mut output = vec![0.0f32; frame.len()];
pipeline.process(&frame, &mut output);
```

The core crate supports `no_std` (disable the default `std` feature).

### CLI

**Process a WAV file:**

```bash
cargo run --release -p stadium-eq-cli -- input.wav output.wav
```

**Real-time microphone processing:**

```bash
cargo run --release -p stadium-eq-cli
```

**Options:**

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

### WASM C-ABI (Low-Level)

If you need direct WASM access without the JS wrapper:

```bash
rustup target add wasm32-unknown-unknown
cargo build -p stadium-eq-web --target wasm32-unknown-unknown --release
```

The `.wasm` binary is written to `target/wasm32-unknown-unknown/release/stadium_eq_web.wasm`.

Exported functions:

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

## Project Structure

```
stadium-equalizer/
  core/       stadium-eq-core     DSP algorithms, pipeline, filters, FFT, STFT
  neural/     stadium-eq-neural   GRU model, feature extraction, band gains
  cli/        stadium-eq-cli      Command-line interface (file + real-time modes)
  web/        stadium-eq-web      WebAssembly C-ABI bindings
  tests/      stadium-eq-tests    Integration tests
  packages/   JS/TS wrappers
    stadium-eq-js/                Vanilla JS/TS wrapper (StadiumEQ class)
    stadium-eq-react/             React hooks and components
  web-ui/                         Browser frontend
```

## Development

### Prerequisites

- [Rust](https://rustup.rs/) stable toolchain
- Linux: `sudo apt-get install libasound2-dev` (ALSA headers for `cpal`)
- WASM builds: `rustup target add wasm32-unknown-unknown`

### Run Tests

```bash
cargo test --workspace --exclude stadium-eq-web
cd packages/stadium-eq-js && npm test
cd packages/stadium-eq-react && npm test
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
