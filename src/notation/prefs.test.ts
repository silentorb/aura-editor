import { describe, expect, test } from "bun:test";
import {
  createPrefsStore,
  DEFAULT_PIANO_PREFS,
  pianoEntryToPrefs,
  pianoPrefsToEntry,
} from "./prefs";

describe("piano-roll prefs", () => {
  test("entries round-trip and bad fields fall back", () => {
    const prefs = { ...DEFAULT_PIANO_PREFS, tool: "erase" as const, click: false };
    expect(pianoEntryToPrefs(pianoPrefsToEntry(prefs))).toEqual(prefs);
    expect(
      pianoEntryToPrefs({ activeTool: "lasso", snapEnabled: false, snapResolution: "1/3" }),
    ).toEqual({ ...DEFAULT_PIANO_PREFS, snapEnabled: false });
    expect(pianoEntryToPrefs(null)).toBe(DEFAULT_PIANO_PREFS);
  });

  test("the store loads once, notifies, and saves changes", async () => {
    const saved: unknown[] = [];
    const store = createPrefsStore(
      async () => ({ activeTool: "select", snapEnabled: true, snapResolution: "1/8" }),
      async (entry) => {
        saved.push(entry);
      },
    );
    let notified = 0;
    store.subscribe(() => {
      notified += 1;
    });
    await Promise.resolve();
    await Promise.resolve();
    expect(store.get().tool).toBe("select");
    store.set({ ...store.get(), tool: "draw" });
    expect(notified).toBe(2);
    expect(saved).toEqual([pianoPrefsToEntry(store.get())]);
  });
});
