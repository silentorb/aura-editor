/**
 * Builds the UI library Imp Lab loads from the Aura pack: one self-contained ES module
 * (`dist/aura_editor.js`) exporting `implementations`.
 *
 * React and Mantine are not bundled. Imp Lab shares its own instances on
 * `globalThis[SHARED_GLOBAL]`; imports of the shared module ids are rewritten to read them
 * from there (named exports are enumerated from the real packages at build time). CSS is
 * inlined into the module and injected when it loads.
 */

import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import { SHARED_GLOBAL, SHARED_MODULE_IDS } from "imp-lab-ui";

const VIRTUAL = "\0imp-lab-shared:";
const IDENTIFIER = /^[A-Za-z_$][\w$]*$/;

function sharedModules(): Plugin {
  const ids = new Set<string>(SHARED_MODULE_IDS);
  return {
    name: "imp-lab-shared-modules",
    enforce: "pre",
    resolveId(source) {
      return ids.has(source) ? VIRTUAL + source : null;
    },
    async load(id) {
      if (!id.startsWith(VIRTUAL)) return null;
      const name = id.slice(VIRTUAL.length);
      const real = (await import(name)) as Record<string, unknown>;
      const names = Object.keys(real).filter((k) => k !== "default" && IDENTIFIER.test(k));
      const lines = [
        `const m = globalThis[${JSON.stringify(SHARED_GLOBAL)}]?.[${JSON.stringify(name)}];`,
        `if (!m) throw new Error(${JSON.stringify(`Imp Lab does not share ${name}`)});`,
        `export default m.default ?? m;`,
        ...names.map((n) => `export const ${n} = m[${JSON.stringify(n)}];`),
      ];
      return lines.join("\n");
    },
  };
}

function inlineCss(): Plugin {
  return {
    name: "inline-css",
    apply: "build",
    enforce: "post",
    generateBundle(_options, bundle) {
      let css = "";
      for (const [file, item] of Object.entries(bundle)) {
        if (item.type === "asset" && file.endsWith(".css")) {
          css += String(item.source);
          delete bundle[file];
        }
      }
      if (!css) return;
      for (const item of Object.values(bundle)) {
        if (item.type === "chunk" && item.isEntry) {
          item.code =
            `(() => { const s = document.createElement("style"); s.dataset.extension = "aura-editor"; ` +
            `s.textContent = ${JSON.stringify(css)}; document.head.appendChild(s); })();\n` +
            item.code;
        }
      }
    },
  };
}

export default defineConfig({
  plugins: [sharedModules(), react(), inlineCss()],
  build: {
    lib: {
      entry: "src/index.ts",
      formats: ["es"],
      fileName: () => "aura_editor.js",
    },
    cssCodeSplit: false,
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
  },
});
