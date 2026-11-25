/** Piano-roll prefs, persisted as the editor component's own UI state. */

import type { PianoRollUiPrefs } from "./PianoRollPanel";

export const DEFAULT_PIANO_PREFS: PianoRollUiPrefs = {
  tool: "draw",
  snapEnabled: true,
  snapResolution: "1/16",
  audition: true,
  click: true,
};

/** Stored shape. */
export type PianoRollUiEntry = {
  activeTool: string;
  snapEnabled: boolean;
  snapResolution: string;
  audition?: boolean;
  click?: boolean;
};

export function pianoPrefsToEntry(prefs: PianoRollUiPrefs): PianoRollUiEntry {
  return {
    activeTool: prefs.tool,
    snapEnabled: prefs.snapEnabled,
    snapResolution: prefs.snapResolution,
    audition: prefs.audition,
    click: prefs.click,
  };
}

export function pianoEntryToPrefs(
  entry: PianoRollUiEntry | null | undefined,
  fallback: PianoRollUiPrefs = DEFAULT_PIANO_PREFS,
): PianoRollUiPrefs {
  if (!entry || typeof entry !== "object") {
    return fallback;
  }
  const tool =
    entry.activeTool === "draw" || entry.activeTool === "select" || entry.activeTool === "erase"
      ? entry.activeTool
      : fallback.tool;
  const snapResolution =
    entry.snapResolution === "1/4" ||
    entry.snapResolution === "1/8" ||
    entry.snapResolution === "1/16" ||
    entry.snapResolution === "1/32"
      ? entry.snapResolution
      : fallback.snapResolution;
  return {
    tool,
    snapEnabled: typeof entry.snapEnabled === "boolean" ? entry.snapEnabled : fallback.snapEnabled,
    snapResolution,
    audition: typeof entry.audition === "boolean" ? entry.audition : fallback.audition,
    click: typeof entry.click === "boolean" ? entry.click : fallback.click,
  };
}

type Listener = () => void;

/** Prefs shared by every open panel of one editor component (they are not per document). */
export type PrefsStore = {
  get(): PianoRollUiPrefs;
  set(next: PianoRollUiPrefs): void;
  subscribe(listener: Listener): () => void;
};

export function createPrefsStore(
  load: () => Promise<PianoRollUiEntry | null>,
  save: (entry: PianoRollUiEntry) => Promise<void>,
): PrefsStore {
  let prefs = DEFAULT_PIANO_PREFS;
  let touched = false;
  const listeners = new Set<Listener>();
  const notify = () => listeners.forEach((l) => l());
  void load()
    .then((entry) => {
      if (!touched && entry) {
        prefs = pianoEntryToPrefs(entry);
        notify();
      }
    })
    .catch((err: unknown) => console.warn("piano-roll prefs unavailable", err));
  return {
    get: () => prefs,
    set(next) {
      touched = true;
      prefs = next;
      notify();
      void save(pianoPrefsToEntry(next)).catch((err: unknown) =>
        console.warn("saving piano-roll prefs failed", err),
      );
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
