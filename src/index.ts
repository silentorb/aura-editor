/**
 * Aura Editor's UI library for Imp Lab: implementations by id. Components in the Aura pack's
 * `extensions.json` choose and configure them; nothing here wires itself in.
 */

import type { UiImplementation } from "imp-lab-ui";
import { notationEditor } from "./notation/notationEditor";

export const implementations: Record<string, UiImplementation> = {
  "notation-editor": notationEditor,
};
