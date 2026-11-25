/**
 * Live note audition for the piano roll.
 *
 * Sends note on / off events to the playback service (when the editor is configured with one) so
 * a note sounds for as long as the user holds it. Lab does not interpret the events; the
 * service decides how notes sound (Aura renders them in its streaming mode).
 */

/** Sends one service event (opaque JSON) to the service that voices audition. */
export type AuditionSend = (event: Record<string, unknown>) => Promise<unknown>;

export type AuditionController = {
  /** Start a note; returns its id (or `null` when audition is unavailable). */
  noteOn(semitone: number, velocity?: number): number | null;
  /** Release a note started by `noteOn` / `retrigger`. */
  noteOff(id: number | null): void;
  /**
   * Move a held note to `semitone`: no-op when the pitch is unchanged, otherwise release it
   * and start the new pitch. Returns the id now sounding.
   */
  retrigger(id: number | null, semitone: number, velocity?: number): number | null;
  /** Release every note. */
  allOff(): void;
  /** False once a send failed (no device, no pack); audition then stays quiet. */
  readonly available: boolean;
};

export function createAuditionController(send: AuditionSend): AuditionController {
  let nextId = 1;
  let available = true;
  const pitches = new Map<number, number>();

  const post = (event: Record<string, unknown>) => {
    if (!available) {
      return;
    }
    send(event).catch((err: unknown) => {
      if (available) {
        available = false;
        console.warn("note audition unavailable:", err);
      }
    });
  };

  const noteOn = (semitone: number, velocity = 1): number | null => {
    if (!available) {
      return null;
    }
    const id = nextId;
    nextId += 1;
    pitches.set(id, semitone);
    post({ type: "note_on", id, semitone, velocity });
    return id;
  };

  const noteOff = (id: number | null) => {
    if (id == null || !pitches.delete(id)) {
      return;
    }
    post({ type: "note_off", id });
  };

  return {
    noteOn,
    noteOff,
    retrigger(id, semitone, velocity = 1) {
      if (id != null && pitches.get(id) === semitone) {
        return id;
      }
      noteOff(id);
      return noteOn(semitone, velocity);
    },
    allOff() {
      if (pitches.size === 0) {
        return;
      }
      pitches.clear();
      post({ type: "all_notes_off" });
    },
    get available() {
      return available;
    },
  };
}
