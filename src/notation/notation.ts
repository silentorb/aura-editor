/** Aura notation record value (`type: "notation"`); same schema the engine loads. */

export type NotationTimeSignature = {
  beatsPerBar: number;
  beatUnit: number;
};

export type NotationNote = {
  semitone: number;
  startBeats: number;
  durationBeats: number;
  velocity: number;
};

export type Notation = {
  type: "notation";
  id?: string;
  title?: string;
  version: 1;
  tempoBpm: number;
  timeSignature: NotationTimeSignature;
  events: NotationNote[];
};

export type SnapResolution = "1/4" | "1/8" | "1/16" | "1/32";

export type NotationTool = "draw" | "select" | "erase";

export const SNAP_RESOLUTIONS: SnapResolution[] = [
  "1/4",
  "1/8",
  "1/16",
  "1/32",
];

export function snapResolutionBeats(resolution: SnapResolution): number {
  switch (resolution) {
    case "1/4":
      return 0.25;
    case "1/8":
      return 0.125;
    case "1/16":
      return 0.0625;
    case "1/32":
      return 0.03125;
  }
}

/** Snaps `value` down to the nearest multiple of `step` when snap is enabled. */
export function snapBeats(
  value: number,
  step: number,
  enabled: boolean,
): number {
  if (!enabled || step <= 0 || !Number.isFinite(value)) {
    return Math.max(0, value);
  }
  return Math.max(0, Math.floor(value / step) * step);
}

export function emptyNotation(id?: string): Notation {
  return {
    type: "notation",
    id: id ?? crypto.randomUUID(),
    version: 1,
    tempoBpm: 120,
    timeSignature: { beatsPerBar: 4, beatUnit: 4 },
    events: [],
  };
}

export function notationNote(
  semitone: number,
  startBeats: number,
  durationBeats: number,
  velocity = 1,
): NotationNote {
  return { semitone, startBeats, durationBeats, velocity };
}

const PITCH_NAMES = [
  "C",
  "C#",
  "D",
  "D#",
  "E",
  "F",
  "F#",
  "G",
  "G#",
  "A",
  "A#",
  "B",
] as const;

/** Scientific pitch label (60 → `C4`). */
export function semitoneLabel(semitone: number): string {
  const pitchClass = ((semitone % 12) + 12) % 12;
  const octave = Math.floor(semitone / 12) - 1;
  return `${PITCH_NAMES[pitchClass]}${octave}`;
}

export function isBlackKey(semitone: number): boolean {
  const pc = ((semitone % 12) + 12) % 12;
  return pc === 1 || pc === 3 || pc === 6 || pc === 8 || pc === 10;
}

export function summarizeNotation(notation: Notation): string {
  const n = notation.events.length;
  return `Notation · ${n} note${n === 1 ? "" : "s"} · ${notation.tempoBpm} BPM`;
}

/** Beat → seconds at notation tempo. */
export function beatsToSeconds(beats: number, tempoBpm: number): number {
  if (!(tempoBpm > 0) || !Number.isFinite(beats)) {
    return 0;
  }
  return (beats * 60) / tempoBpm;
}

/** Seconds → beats at notation tempo. */
export function secondsToBeats(secs: number, tempoBpm: number): number {
  if (!(tempoBpm > 0) || !Number.isFinite(secs)) {
    return 0;
  }
  return (secs * tempoBpm) / 60;
}

/** End of score in beats (max start+duration). */
export function notationDurationBeats(notation: Notation): number {
  let max = 0;
  for (const note of notation.events) {
    max = Math.max(max, note.startBeats + note.durationBeats);
  }
  return max;
}

/**
 * Parse / validate a JSON value as Notation.
 * Unknown extra fields are dropped by reconstructing a typed object.
 */
export function notationFromUnknown(value: unknown): Notation {
  if (!value || typeof value !== "object") {
    throw new Error("notation must be an object");
  }
  const raw = value as Record<string, unknown>;
  if (raw.type !== "notation") {
    throw new Error(`expected type "notation", got ${JSON.stringify(raw.type)}`);
  }
  if (raw.version !== 1) {
    throw new Error(`unsupported notation version ${String(raw.version)}`);
  }
  const tempoBpm = Number(raw.tempoBpm);
  if (!(tempoBpm > 0)) {
    throw new Error("notation requires a positive tempoBpm");
  }
  const ts = raw.timeSignature;
  if (!ts || typeof ts !== "object") {
    throw new Error("notation requires timeSignature");
  }
  const timeSignature = ts as Record<string, unknown>;
  const beatsPerBar = Number(timeSignature.beatsPerBar);
  const beatUnit = Number(timeSignature.beatUnit);
  if (!(beatsPerBar > 0) || !(beatUnit > 0)) {
    throw new Error("invalid timeSignature");
  }
  const eventsRaw = Array.isArray(raw.events) ? raw.events : [];
  const events: NotationNote[] = eventsRaw.map((item, index) => {
    if (!item || typeof item !== "object") {
      throw new Error(`events[${index}] must be an object`);
    }
    const note = item as Record<string, unknown>;
    const semitone = Number(note.semitone);
    const startBeats = Number(note.startBeats);
    const durationBeats = Number(note.durationBeats);
    const velocity =
      note.velocity === undefined ? 1 : Number(note.velocity);
    if (!Number.isFinite(semitone) || !Number.isFinite(startBeats) || !Number.isFinite(durationBeats)) {
      throw new Error(`events[${index}] has invalid numeric fields`);
    }
    return {
      semitone,
      startBeats,
      durationBeats,
      velocity: Number.isFinite(velocity) ? velocity : 1,
    };
  });

  const notation: Notation = {
    type: "notation",
    version: 1,
    tempoBpm,
    timeSignature: { beatsPerBar, beatUnit },
    events,
  };
  if (typeof raw.id === "string" && raw.id.length > 0) {
    notation.id = raw.id;
  }
  if (typeof raw.title === "string") {
    notation.title = raw.title;
  }
  return notation;
}

/** Stable serialization of what a notation means (dirty tracking and undo). */
export function notationFingerprint(notation: Notation): string {
  return JSON.stringify({
    id: notation.id ?? null,
    title: notation.title ?? null,
    tempoBpm: notation.tempoBpm,
    timeSignature: notation.timeSignature,
    events: notation.events,
  });
}

export function notationsEqual(a: Notation, b: Notation): boolean {
  return notationFingerprint(a) === notationFingerprint(b);
}
