/** Pure notation edit operations (draw / move / resize / erase). */

import {
  notationNote,
  type Notation,
  type NotationNote,
} from "./notation";

export type NotationEdit =
  | {
      kind: "draw";
      semitone: number;
      startBeats: number;
      durationBeats: number;
    }
  | { kind: "erase"; noteIndex: number }
  | {
      kind: "move";
      noteIndex: number;
      startBeats: number;
      semitone: number;
    }
  | {
      kind: "resize";
      noteIndex: number;
      startBeats: number;
      durationBeats: number;
    };

/** Applies an edit; returns a new notation when dirty, else the same reference. */
export function applyNotationEdit(
  notation: Notation,
  edit: NotationEdit,
): Notation {
  switch (edit.kind) {
    case "draw": {
      const duplicate = notation.events.some(
        (note) =>
          note.semitone === edit.semitone &&
          Math.abs(note.startBeats - edit.startBeats) < 1e-9 &&
          Math.abs(note.durationBeats - edit.durationBeats) < 1e-9,
      );
      if (duplicate) {
        return notation;
      }
      return {
        ...notation,
        events: [
          ...notation.events,
          notationNote(edit.semitone, edit.startBeats, edit.durationBeats),
        ],
      };
    }
    case "erase": {
      if (edit.noteIndex < 0 || edit.noteIndex >= notation.events.length) {
        return notation;
      }
      return {
        ...notation,
        events: notation.events.filter((_, i) => i !== edit.noteIndex),
      };
    }
    case "move": {
      const note = notation.events[edit.noteIndex];
      if (!note) {
        return notation;
      }
      if (
        Math.abs(note.startBeats - edit.startBeats) < 1e-9 &&
        note.semitone === edit.semitone
      ) {
        return notation;
      }
      return {
        ...notation,
        events: notation.events.map((n, i) =>
          i === edit.noteIndex
            ? { ...n, startBeats: edit.startBeats, semitone: edit.semitone }
            : n,
        ),
      };
    }
    case "resize": {
      const note = notation.events[edit.noteIndex];
      if (!note) {
        return notation;
      }
      if (
        Math.abs(note.startBeats - edit.startBeats) < 1e-9 &&
        Math.abs(note.durationBeats - edit.durationBeats) < 1e-9
      ) {
        return notation;
      }
      const durationBeats = Math.max(1e-6, edit.durationBeats);
      return {
        ...notation,
        events: notation.events.map((n, i) =>
          i === edit.noteIndex
            ? { ...n, startBeats: edit.startBeats, durationBeats }
            : n,
        ),
      };
    }
  }
}

export function cloneNote(note: NotationNote): NotationNote {
  return { ...note };
}
