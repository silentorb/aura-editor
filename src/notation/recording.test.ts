import { describe, expect, test } from "bun:test";
import { emptyNotation, notationNote } from "./notation";
import {
  applyTake,
  createRecordingController,
  notesFromUnknown,
  takeFromTopic,
  type CaptureRequest,
} from "./recording";

function service(replies: Record<string, unknown>, fail = false) {
  const requests: Record<string, unknown>[] = [];
  const invoke: CaptureRequest = async (body) => {
    requests.push(body);
    if (fail) {
      throw new Error("request unsupported");
    }
    return JSON.stringify(replies[body.type as string] ?? {});
  };
  return { invoke, requests };
}

describe("recording", () => {
  test("a take overdubs as one edit, ordered by start", () => {
    const notation = {
      ...emptyNotation("n"),
      events: [notationNote(60, 0, 1), notationNote(64, 2, 1)],
    };
    const next = applyTake(notation, [notationNote(62, 1, 0.5, 0.75)]);
    expect(next.events.map((e) => e.semitone)).toEqual([60, 62, 64]);
    expect(notation.events.length).toBe(2);
    expect(applyTake(notation, [])).toBe(notation);
  });

  test("notes and take topics are validated", () => {
    expect(
      notesFromUnknown([
        { semitone: 60, startBeats: -1, durationBeats: 1, velocity: 2 },
        { semitone: 61, startBeats: 0, durationBeats: 0, velocity: 1 },
        "x",
      ]),
    ).toEqual([{ semitone: 60, startBeats: 0, durationBeats: 1, velocity: 1 }]);
    expect(takeFromTopic(undefined).recording).toBe(false);
    expect(
      takeFromTopic({ recording: true, countIn: true, notes: [] }),
    ).toEqual({ recording: true, countIn: true, notes: [] });
  });

  test("controller speaks the capture protocol", async () => {
    const { invoke, requests } = service({
      capture_info: { topic: "aura.capture.take" },
      record_start: { topic: "aura.capture.take" },
      record_stop: {
        notes: [{ semitone: 60, startBeats: 4, durationBeats: 1, velocity: 0.5 }],
      },
    });
    const recording = createRecordingController(invoke);
    expect(await recording.info()).toBe("aura.capture.take");
    const notation = emptyNotation("n");
    expect(await recording.start(notation, 4, true)).toBe("aura.capture.take");
    expect(requests[1]).toEqual({
      type: "record_start",
      notation,
      startBeats: 4,
      click: true,
    });
    expect(await recording.stop()).toEqual([
      { semitone: 60, startBeats: 4, durationBeats: 1, velocity: 0.5 },
    ]);
  });

  test("no capture service: info is null", async () => {
    const { invoke } = service({}, true);
    expect(await createRecordingController(invoke).info()).toBeNull();
  });
});
