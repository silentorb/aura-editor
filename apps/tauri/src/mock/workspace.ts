/** Mock workspace state for the Session 1 UI prototype (no host/engine wiring). */

export type DocumentKind = "imp-graph" | "notation";

export type MockDocument = {
  id: string;
  title: string;
  kind: DocumentKind;
  /** Display-only path / origin label */
  origin: string;
  dirty: boolean;
  summary: string;
  properties: { label: string; value: string }[];
};

export const MOCK_DOCUMENTS: MockDocument[] = [
  {
    id: "doc-graph-01",
    title: "demo-01",
    kind: "imp-graph",
    origin: "file:…/demos/demo-01.json",
    dirty: false,
    summary:
      "Mock Imp graph · 12 nodes · 14 edges · output “out”. Graph-dag canvas not ported yet — placeholder content only.",
    properties: [
      { label: "Id", value: "01JMOCKGRAPH000000000001" },
      { label: "Type", value: "imp-graph" },
      { label: "Title", value: "demo-01" },
      { label: "Nodes", value: "12" },
    ],
  },
  {
    id: "doc-notation-01",
    title: "motif-a",
    kind: "notation",
    origin: "miro:…/content · NodeId 01JMOCKNOTE000000000001",
    dirty: true,
    summary:
      "Mock notation · 8 events · 120 BPM · 4/4. Piano-roll view not ported yet — placeholder content only.",
    properties: [
      { label: "Id", value: "01JMOCKNOTE000000000001" },
      { label: "Type", value: "notation" },
      { label: "Title", value: "motif-a" },
      { label: "Events", value: "8" },
      { label: "Tempo", value: "120" },
    ],
  },
];

export type TransportState = "stopped" | "playing";
