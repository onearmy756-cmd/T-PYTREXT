import { resolve } from "node:path";
import { defineConfig, type Plugin } from "vite";

// The Canvas host requires exactly one self-contained, statically-importable
// ESM entrypoint. Two adjustments to the emitted bundle are therefore needed:
//   1. Monaco ships CSS that Vite extracts into a sibling file -> inline it.
//   2. Monaco's worker bootstrap contains a runtime `import()` (a generated
//      worker script plus foreign-module loading). Neither path runs for the
//      highlight-and-edit usage here; both break Blob-ESM loading -> rewrite
//      them into static-safe forms that never call a bare dynamic import.
function canvasEntrypoint(): Plugin {
  return {
    name: "mbide-canvas-entrypoint",
    enforce: "post",
    generateBundle(_options, bundle) {
      let css = "";
      for (const [file, asset] of Object.entries(bundle)) {
        if (file.endsWith(".css") && asset.type === "asset") {
          css += String(asset.source);
          delete bundle[file];
        }
      }
      const entry = Object.values(bundle).find((c) => c.type === "chunk" && c.isEntry);
      if (!entry || entry.type !== "chunk") return;

      entry.code = entry.code.replace(/import\(`\$\{url\}`\)/g, "Promise.reject(new Error('foreign modules disabled'))");
      entry.code = entry.code.replace(/`(\s*)await import\(ttPolicy/g, "`$1await (self.__mbNoImport||function(){throw new Error('worker bootstrap disabled')})(ttPolicy");

      if (css) {
        entry.code +=
          `\n;(function(){try{var s=document.createElement("style");` +
          `s.setAttribute("data-mbilinyitech-ide-css","");s.textContent=${JSON.stringify(css)};` +
          `document.head.appendChild(s);}catch(e){}})();\n`;
      }
    },
  };
}

// One self-contained browser ESM entrypoint (dist/extension.js), Blob-importable
// by Agent Canvas. No code splitting, no sibling assets, bundled CSS.
export default defineConfig({
  plugins: [canvasEntrypoint()],
  build: {
    assetsInlineLimit: Number.MAX_SAFE_INTEGER,
    cssCodeSplit: false,
    emptyOutDir: true,
    lib: {
      entry: resolve(import.meta.dirname, "src/extension.ts"),
      formats: ["es"],
      fileName: () => "extension.js",
    },
    outDir: "dist",
    rollupOptions: { output: { inlineDynamicImports: true } },
    sourcemap: false,
    minify: false,
  },
});
