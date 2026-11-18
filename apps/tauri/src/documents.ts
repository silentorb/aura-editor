/** Document tab state for the Tauri shell (summary DTOs, not full ModelGraph). */

export type DocumentKind = "imp-graph" | "notation";

export type DocumentProperty = { label: string; value: string };

export type EditorDocument = {
  id: string;
  title: string;
  kind: DocumentKind;
  /** Filesystem path or untitled label */
  origin: string;
  dirty: boolean;
  summary: string;
  properties: DocumentProperty[];
};

export type TransportState = "stopped" | "playing";

/** Host DTO from `pick_and_open_document` / `open_document_file`. */
export type DocumentSummaryDto = {
  id: string;
  title: string;
  kind: DocumentKind;
  origin: string;
  dirty: boolean;
  summary: string;
  properties: DocumentProperty[];
};

export function documentFromDto(dto: DocumentSummaryDto): EditorDocument {
  return { ...dto };
}

export function newUntitledDocument(kind: DocumentKind): EditorDocument {
  const id = crypto.randomUUID();
  const title = kind === "imp-graph" ? "Untitled graph" : "Untitled notation";
  return {
    id,
    title,
    kind,
    origin: "untitled",
    dirty: true,
    summary:
      kind === "imp-graph"
        ? "Untitled Imp graph · empty. Graph-dag canvas not ported yet — placeholder content only."
        : "Untitled notation · empty. Piano-roll view not ported yet — placeholder content only.",
    properties: [
      { label: "Id", value: id },
      { label: "Type", value: kind },
      { label: "Title", value: title },
    ],
  };
}
