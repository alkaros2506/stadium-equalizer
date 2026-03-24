# Stadium EQ — Vercel Demo

Interactive demo showcasing 3 ways to integrate the Stadium Audio Equalizer:

| Route | Wrapper | Description |
|-------|---------|-------------|
| `/` | `<StadiumEqualizer>` | Drop-in React component — one import, zero config |
| `/custom` | `useStadiumEQ()` | React hook with fully custom UI (circular visualizer) |
| `/vanilla` | `new StadiumEQ()` | Vanilla JS class with event log — no framework needed |

## Quick start

```bash
# 1. Build the WASM binary
cargo build -p stadium-eq-web --target wasm32-unknown-unknown --release

# 2. Copy it to public/
cp ../../target/wasm32-unknown-unknown/release/stadium_eq_web.wasm public/stadium_eq.wasm

# 3. Install & run
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000).

## Deploy to Vercel

1. Set the **Root Directory** to `examples/demo` in Vercel project settings.
2. Ensure `public/stadium_eq.wasm` is committed (or use a build step to copy it).
3. Deploy — Vercel handles the rest. CORS headers for WASM are configured in `vercel.json`.
