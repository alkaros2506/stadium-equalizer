import { useState, useEffect, useRef, useCallback } from "react";
import { StadiumEQ } from "stadium-eq";
import type { StadiumEQOptions, MixLevels, PipelineStatus } from "stadium-eq";

export interface UseStadiumEQOptions
  extends Pick<StadiumEQOptions, "wasmUrl" | "workletUrl" | "sampleRate" | "frameSize"> {}

export interface UseStadiumEQReturn {
  /** Current pipeline status. */
  status: PipelineStatus;
  /** Whether the pipeline is actively processing audio. */
  isRunning: boolean;
  /** Start the audio pipeline. */
  start: () => Promise<void>;
  /** Stop the audio pipeline. */
  stop: () => void;
  /** Start calibration (noise profiling). */
  calibrate: () => void;
  /** Update mix levels. Accepts partial updates. */
  setMix: (mix: Partial<MixLevels>) => void;
  /** Enable or disable bypass mode. */
  setBypass: (bypass: boolean) => void;
  /** Current mix levels. */
  mix: MixLevels;
  /** The underlying StadiumEQ instance, or null if not yet created. */
  instance: StadiumEQ | null;
  /** Last error message, if any. */
  error: string | null;
}

/**
 * React hook for the Stadium Audio Equalizer.
 *
 * @example
 * ```tsx
 * import { useStadiumEQ } from "stadium-eq-react";
 *
 * function App() {
 *   const eq = useStadiumEQ({ wasmUrl: "/stadium_eq.wasm" });
 *
 *   return (
 *     <div>
 *       <p>Status: {eq.status}</p>
 *       <button onClick={eq.isRunning ? eq.stop : eq.start}>
 *         {eq.isRunning ? "Stop" : "Start"}
 *       </button>
 *       <button onClick={eq.calibrate} disabled={!eq.isRunning}>
 *         Calibrate
 *       </button>
 *       <input
 *         type="range" min="-1" max="1" step="0.01"
 *         value={eq.mix.crowd}
 *         onChange={e => eq.setMix({ crowd: Number(e.target.value) })}
 *       />
 *     </div>
 *   );
 * }
 * ```
 */
export function useStadiumEQ(options: UseStadiumEQOptions): UseStadiumEQReturn {
  const [status, setStatus] = useState<PipelineStatus>("idle");
  const [mix, setMixState] = useState<MixLevels>({
    crowd: 0,
    speaker: 0,
    music: 0,
    gainDb: 0,
  });
  const [error, setError] = useState<string | null>(null);
  const eqRef = useRef<StadiumEQ | null>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  // Create the instance lazily on first start
  const getOrCreateInstance = useCallback((): StadiumEQ => {
    if (!eqRef.current) {
      const eq = new StadiumEQ(optionsRef.current);
      eq.on("statuschange", setStatus);
      eq.on("error", (err: string) => setError(err));
      eqRef.current = eq;
    }
    return eqRef.current;
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      eqRef.current?.destroy();
      eqRef.current = null;
    };
  }, []);

  const start = useCallback(async () => {
    setError(null);
    const eq = getOrCreateInstance();
    await eq.start();
  }, [getOrCreateInstance]);

  const stop = useCallback(() => {
    eqRef.current?.stop();
  }, []);

  const calibrate = useCallback(() => {
    eqRef.current?.calibrate();
  }, []);

  const setMix = useCallback((update: Partial<MixLevels>) => {
    setMixState((prev: MixLevels) => {
      const next = { ...prev, ...update };
      eqRef.current?.setMix(next);
      return next;
    });
  }, []);

  const setBypass = useCallback((bypass: boolean) => {
    eqRef.current?.setBypass(bypass);
  }, []);

  const isRunning =
    status === "processing" ||
    status === "calibrating" ||
    status === "bypassed";

  return {
    status,
    isRunning,
    start,
    stop,
    calibrate,
    setMix,
    setBypass,
    mix,
    instance: eqRef.current,
    error,
  };
}
