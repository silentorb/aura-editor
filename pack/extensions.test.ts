/**
 * The Aura pack configuration wires engine extensions (catalogs, playback) and Aura Editor's UI
 * library together. These checks keep its references consistent with each other and with the
 * ids the engine extensions and the editor implementations use.
 */

import { describe, expect, test } from "bun:test";
import { implementations } from "../src/index";
import { CMD_PLAY_TOGGLE, CMD_RECORD_TOGGLE } from "../src/notation/notationEditor";

type Component = {
  id: string;
  extensionId: string;
  kind: string;
  implementationId?: string;
  enabled?: boolean;
  label?: string;
  params?: Record<string, unknown>;
};

type Pack = {
  version: number;
  extensions: Array<{ id: string; libraryPath?: string; uiLibraryPath?: string }>;
  components: Component[];
};

const pack = (await Bun.file(new URL("./extensions.json", import.meta.url)).json()) as Pack;
const of = (kind: string) => pack.components.filter((c) => c.kind === kind);
const services = new Map(of("service").map((c) => [c.id, c.implementationId]));

/** Engine-side ids (aura_imp_lab_playback). */
const PLAYBACK_SERVICE = "playback";
const MIDI_SETTINGS = ["aura.midi.input", "aura.midi.channels"];
const MIDI_STATUS_TOPIC = "aura.midi.status";
const PLUGIN_TOPICS = [
  ["plugin_instrument", "aura.plugins.instruments"],
  ["plugin_effect", "aura.plugins.effects"],
];
const PLUGIN_EDITS_TOPIC = "aura.plugins.edits";
const OPEN_GUI = "plugin.open_gui";
const NOTATION_NODE_TYPE = "01M80M9N0TAT00000000000001";

describe("Aura pack extensions.json", () => {
  test("every component belongs to a declared extension and ids are unique", () => {
    expect(pack.version).toBe(1);
    const extensions = new Set(pack.extensions.map((e) => e.id));
    for (const c of pack.components) {
      expect(extensions.has(c.extensionId)).toBe(true);
    }
    const ids = pack.components.map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  test("service references resolve to the playback service", () => {
    expect([...services.values()]).toContain(PLAYBACK_SERVICE);
    const referencing = pack.components.filter(
      (c) => c.kind !== "service" && typeof c.params?.serviceId === "string",
    );
    expect(referencing.length).toBeGreaterThan(0);
    for (const c of referencing) {
      expect(services.get(c.params!.serviceId as string)).toBe(PLAYBACK_SERVICE);
    }
  });

  test("MIDI settings sit on the MIDI page, with a status item", () => {
    expect(of("setting").map((s) => s.id)).toEqual(MIDI_SETTINGS);
    const items = of("settings-category-item").map((i) => [i.params?.settingId, i.params?.categoryId]);
    expect(items).toEqual(MIDI_SETTINGS.map((s) => [s, "aura.midi"]));
    expect(of("settings-category").map((c) => c.id)).toEqual(["aura.midi"]);
    expect(of("status-item").map((s) => s.params?.topic)).toEqual([MIDI_STATUS_TOPIC]);
  });

  test("plugin nodes get choices and GUI actions", () => {
    expect(of("node-literal-options").map((o) => [o.params?.nodeType, o.params?.optionsTopic])).toEqual(
      PLUGIN_TOPICS,
    );
    const actions = of("node-action");
    expect(actions).toHaveLength(2);
    for (const a of actions) {
      expect(a.params?.action).toBe(OPEN_GUI);
      expect(a.params?.editsTopic).toBe(PLUGIN_EDITS_TOPIC);
    }
  });

  test("the notation editor is wired to the Notations node type and its UI library", () => {
    const editors = of("node-type-editor");
    expect(editors).toHaveLength(1);
    const editor = editors[0]!;
    expect(editor.params?.nodeTypeId).toBe(NOTATION_NODE_TYPE);
    expect(editor.params?.column).toBe("notation");
    expect(editor.params?.documentType).toBe("notation");
    expect(implementations[editor.implementationId!]?.kind).toBe("node-type-editor");
    const library = pack.extensions.find((e) => e.id === editor.extensionId);
    expect(library?.uiLibraryPath).toBe("extensions/aura_editor.js");
  });

  test("bindings name commands the pack or the editor handles", () => {
    const commands = new Set(of("command").map((c) => c.implementationId));
    const bound = of("binding").map((b) => b.params?.command as string);
    expect(bound).toContain(CMD_PLAY_TOGGLE);
    expect(bound).toContain(CMD_RECORD_TOGGLE);
    for (const command of bound) {
      expect(commands.has(command) || command.startsWith("notation.")).toBe(true);
    }
  });
});
