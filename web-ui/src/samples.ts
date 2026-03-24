// Predefined audio sample definitions for the demo.
//
// Each sample simulates a different stadium / live-event environment so
// users can test the equalizer controls without a live microphone.

export interface AudioSample {
  id: string;
  label: string;
  description: string;
  /** Path relative to the web root (served from public/). */
  file: string;
}

export const AUDIO_SAMPLES: AudioSample[] = [
  {
    id: "stadium-announcer",
    label: "Stadium Announcer",
    description:
      "PA-style announcer voice over a steady stadium crowd hum — typical of pre-game or half-time.",
    file: "samples/stadium-announcer.ogg",
  },
  {
    id: "football-match",
    label: "Football Match",
    description:
      "Outdoor football crowd with cheering, chanting, and referee whistles.",
    file: "samples/football-match.ogg",
  },
  {
    id: "concert-venue",
    label: "Concert Venue",
    description:
      "Large concert hall crowd with bass-heavy music and an MC speaking through reverb.",
    file: "samples/concert-venue.ogg",
  },
  {
    id: "basketball-arena",
    label: "Basketball Arena",
    description:
      "Indoor arena echo with sneaker squeaks, a buzzer, and bursty crowd reactions.",
    file: "samples/basketball-arena.ogg",
  },
  {
    id: "rally-outdoor",
    label: "Rally / Outdoor Event",
    description:
      "Open-air crowd with wind noise and a megaphone-style speaker addressing the audience.",
    file: "samples/rally-outdoor.ogg",
  },
];
