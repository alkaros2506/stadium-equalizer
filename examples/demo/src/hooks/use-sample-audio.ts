"use client";

import { useRef, useCallback } from "react";
import { AUDIO_SAMPLES } from "../audio-samples";

/**
 * Manages an HTMLAudioElement for sample playback.
 * Returns the element (or null for mic) and cleanup helpers.
 */
export function useSampleAudio() {
  const audioRef = useRef<HTMLAudioElement | null>(null);

  /** Create and return an <audio> element for the given source id, or null for "mic". */
  const getAudioElement = useCallback(
    (sourceId: string): HTMLAudioElement | undefined => {
      // Clean up any previous element
      cleanup();

      if (sourceId === "mic") return undefined;

      const sample = AUDIO_SAMPLES.find((s) => s.id === sourceId);
      if (!sample) return undefined;

      const el = new Audio(sample.file);
      el.crossOrigin = "anonymous";
      el.loop = true;
      audioRef.current = el;
      return el;
    },
    []
  );

  const cleanup = useCallback(() => {
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.src = "";
      audioRef.current = null;
    }
  }, []);

  /** Start playback of the current audio element (no-op if mic). */
  const play = useCallback(() => {
    audioRef.current?.play();
  }, []);

  return { getAudioElement, play, cleanup };
}
