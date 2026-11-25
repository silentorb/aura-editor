import { describe, expect, test } from "bun:test";
import { createAuditionController, type AuditionSend } from "./audition";

function recorder(fail = false) {
  const events: Record<string, unknown>[] = [];
  const invoke: AuditionSend = async (event) => {
    events.push(event);
    if (fail) {
      throw new Error("no audio device");
    }
  };
  return { events, invoke };
}

describe("audition controller", () => {
  test("note on then off with increasing ids", () => {
    const { events, invoke } = recorder();
    const audition = createAuditionController(invoke);
    const a = audition.noteOn(60, 0.8);
    const b = audition.noteOn(64);
    audition.noteOff(a);
    expect(b).toBe(a! + 1);
    expect(events).toEqual([
      { type: "note_on", id: a, semitone: 60, velocity: 0.8 },
      { type: "note_on", id: b, semitone: 64, velocity: 1 },
      { type: "note_off", id: a },
    ]);
  });

  test("retrigger only when the pitch changes", () => {
    const { events, invoke } = recorder();
    const audition = createAuditionController(invoke);
    const first = audition.noteOn(60);
    expect(audition.retrigger(first, 60)).toBe(first);
    const second = audition.retrigger(first, 62);
    expect(second).not.toBe(first);
    expect(events.map((e) => e.type)).toEqual(["note_on", "note_off", "note_on"]);
  });

  test("note off for unknown or released ids is ignored", () => {
    const { events, invoke } = recorder();
    const audition = createAuditionController(invoke);
    const id = audition.noteOn(60);
    audition.noteOff(id);
    audition.noteOff(id);
    audition.noteOff(null);
    audition.allOff();
    expect(events.map((e) => e.type)).toEqual(["note_on", "note_off"]);
  });

  test("goes quiet after the first failure", async () => {
    const { events, invoke } = recorder(true);
    const audition = createAuditionController(invoke);
    audition.noteOn(60);
    await Promise.resolve();
    await Promise.resolve();
    expect(audition.available).toBe(false);
    expect(audition.noteOn(62)).toBeNull();
    expect(events).toHaveLength(1);
  });
});
