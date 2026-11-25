import { describe, expect, test } from "bun:test";
import { applyNotationEdit } from "./edits";
import { emptyNotation } from "./notation";

describe("applyNotationEdit", () => {
  test("draw appends a note", () => {
    const notation = emptyNotation("t");
    const next = applyNotationEdit(notation, {
      kind: "draw",
      semitone: 60,
      startBeats: 0,
      durationBeats: 0.25,
    });
    expect(next.events).toHaveLength(1);
    expect(next.events[0]?.semitone).toBe(60);
    expect(next).not.toBe(notation);
  });

  test("draw skips identical cell", () => {
    let notation = emptyNotation("t");
    notation = applyNotationEdit(notation, {
      kind: "draw",
      semitone: 60,
      startBeats: 0,
      durationBeats: 0.25,
    });
    const again = applyNotationEdit(notation, {
      kind: "draw",
      semitone: 60,
      startBeats: 0,
      durationBeats: 0.25,
    });
    expect(again).toBe(notation);
  });

  test("erase removes by index", () => {
    let notation = emptyNotation("t");
    notation = applyNotationEdit(notation, {
      kind: "draw",
      semitone: 60,
      startBeats: 0,
      durationBeats: 0.25,
    });
    notation = applyNotationEdit(notation, {
      kind: "draw",
      semitone: 64,
      startBeats: 0.25,
      durationBeats: 0.25,
    });
    const next = applyNotationEdit(notation, { kind: "erase", noteIndex: 0 });
    expect(next.events).toHaveLength(1);
    expect(next.events[0]?.semitone).toBe(64);
  });

  test("move updates start and pitch", () => {
    let notation = emptyNotation("t");
    notation = applyNotationEdit(notation, {
      kind: "draw",
      semitone: 60,
      startBeats: 0,
      durationBeats: 0.25,
    });
    const next = applyNotationEdit(notation, {
      kind: "move",
      noteIndex: 0,
      startBeats: 1,
      semitone: 62,
    });
    expect(next.events[0]?.startBeats).toBe(1);
    expect(next.events[0]?.semitone).toBe(62);
  });

  test("resize updates duration", () => {
    let notation = emptyNotation("t");
    notation = applyNotationEdit(notation, {
      kind: "draw",
      semitone: 60,
      startBeats: 0,
      durationBeats: 0.25,
    });
    const next = applyNotationEdit(notation, {
      kind: "resize",
      noteIndex: 0,
      startBeats: 0,
      durationBeats: 1,
    });
    expect(next.events[0]?.durationBeats).toBe(1);
  });
});
