/**
 * Recording takes into the piano roll through a service's live note capture protocol.
 *
 * The editor doesn't know where the notes come from (the Aura pack records a MIDI input). It
 * starts a take, shows the take topic while it grows, and on stop applies the returned notation notes to
 * the document as one edit (overdub).
 */

import type { Notation, NotationNote } from "./notation";

/** Sends one request to the capture service and resolves to its (parsed) JSON reply. */
export type CaptureRequest = (body: Record<string, unknown>) => Promise<unknown>;

/** Value of the take topic. */
export type TakeState = {
  recording: boolean;
  countIn: boolean;
  notes: NotationNote[];
};

export const IDLE_TAKE: TakeState = { recording: false, countIn: false, notes: [] };

function isNote(value: unknown): value is NotationNote {
  if (!value || typeof value !== "object") {
    return false;
  }
  const n = value as Record<string, unknown>;
  return (
    typeof n.semitone === "number" &&
    typeof n.startBeats === "number" &&
    typeof n.durationBeats === "number" &&
    typeof n.velocity === "number" &&
    Number.isFinite(n.startBeats) &&
    Number.isFinite(n.durationBeats) &&
    n.durationBeats > 0
  );
}

/** Valid notes from an unknown value. */
export function notesFromUnknown(value: unknown): NotationNote[] {
  return Array.isArray(value)
    ? value.filter(isNote).map((n) => ({
        semitone: Math.round(n.semitone),
        startBeats: Math.max(0, n.startBeats),
        durationBeats: n.durationBeats,
        velocity: Math.min(1, Math.max(0, n.velocity)),
      }))
    : [];
}

/** A take topic value, or idle when it isn't one. */
export function takeFromTopic(value: unknown): TakeState {
  if (!value || typeof value !== "object") {
    return IDLE_TAKE;
  }
  const v = value as Record<string, unknown>;
  return {
    recording: v.recording === true,
    countIn: v.countIn === true,
    notes: notesFromUnknown(v.notes),
  };
}

/** Adds a take to the notation (overdub), ordered by start. Unchanged when the take is empty. */
export function applyTake(notation: Notation, notes: NotationNote[]): Notation {
  if (notes.length === 0) {
    return notation;
  }
  const events = [...notation.events, ...notes].sort(
    (a, b) => a.startBeats - b.startBeats || a.semitone - b.semitone,
  );
  return { ...notation, events };
}

export type RecordingController = {
  /** The take topic when the service offers capture, else `null`. */
  info(): Promise<string | null>;
  /** Starts a take from `startBeats`; returns the take topic. */
  start(notation: Notation, startBeats: number, click: boolean): Promise<string>;
  /** Stops the take and returns its notes. */
  stop(): Promise<NotationNote[]>;
};

export function createRecordingController(send: CaptureRequest): RecordingController {
  const request = async (body: Record<string, unknown>): Promise<Record<string, unknown>> => {
    const reply = await send(body);
    const parsed: unknown = typeof reply === "string" ? JSON.parse(reply) : reply;
    return parsed && typeof parsed === "object" ? (parsed as Record<string, unknown>) : {};
  };
  return {
    async info() {
      try {
        const reply = await request({ type: "capture_info" });
        return typeof reply.topic === "string" ? reply.topic : null;
      } catch {
        return null;
      }
    },
    async start(notation, startBeats, click) {
      const reply = await request({
        type: "record_start",
        notation,
        startBeats: Math.max(0, startBeats),
        click,
      });
      if (typeof reply.topic !== "string") {
        throw new Error("capture service did not name a take topic");
      }
      return reply.topic;
    },
    async stop() {
      const reply = await request({ type: "record_stop" });
      return notesFromUnknown(reply.notes);
    },
  };
}
