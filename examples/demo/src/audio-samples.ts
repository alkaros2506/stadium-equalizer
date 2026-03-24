// Predefined audio samples for the demo.

export interface AudioSample {
  id: string;
  label: string;
  description: string;
  file: string;
}

export const AUDIO_SAMPLES: AudioSample[] = [
  {
    id: "stadium-announcer",
    label: "Stadium Roar",
    description:
      "Concert stadium crowd roar with cheering and applause.",
    file: "/samples/stadium-announcer.mp3",
  },
  {
    id: "football-match",
    label: "Football Match",
    description:
      "Real field recording from a soccer/football match with crowd ambience.",
    file: "/samples/football-match.mp3",
  },
  {
    id: "concert-venue",
    label: "Concert Crowd",
    description:
      "Concert/stadium crowd cheering with clapping and applause.",
    file: "/samples/concert-venue.mp3",
  },
  {
    id: "basketball-arena",
    label: "Stadium Cheering",
    description:
      "Stadium crowd cheering and applause from a sports event.",
    file: "/samples/basketball-arena.mp3",
  },
  {
    id: "rally-outdoor",
    label: "Crowd Cheering",
    description:
      "Short burst of crowd cheering at a concert or sporting event.",
    file: "/samples/rally-outdoor.mp3",
  },
];
