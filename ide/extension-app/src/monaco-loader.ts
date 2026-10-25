import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
// Full editor contributions (ghost-text inline completions, Cmd+K inline edits,
// suggest, find, folding, bracket matching, ...). editor.all.js is side-effect
// only; the API namespace stays editor.api.
import "monaco-editor/esm/vs/editor/editor.all";
// Syntax highlighting for the languages MTECHOS ships. The full monaco-editor
// default entrypoint also pulls language-service web workers that use runtime
// dynamic import; the Canvas host requires a single static ESM entrypoint.
import "monaco-editor/esm/vs/basic-languages/monaco.contribution";

let configured = false;

export function defineThemes(): void {
  if (configured) return;
  configured = true;
  monaco.editor.defineTheme("zed-one-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment", foreground: "5c6370", fontStyle: "italic" },
      { token: "keyword", foreground: "c678dd" },
      { token: "string", foreground: "98c379" },
      { token: "number", foreground: "d19a66" },
      { token: "type", foreground: "e5c07b" },
      { token: "function", foreground: "61afef" },
      { token: "variable", foreground: "e06c75" },
    ],
    colors: {
      "editor.background": "#0d1117",
      "editor.foreground": "#abb2bf",
      "editorLineNumber.foreground": "#4b5263",
      "editor.lineHighlightBackground": "#1b2230",
    },
  });
}

export interface CreateEditorOptions {
  host: HTMLElement;
  value: string;
  language: string;
  onChange?: () => void;
  onSave?: () => void;
  onCursor?: (line: number, column: number) => void;
}

const SAVE_KEY = 2048 /* KeyMod.CtrlCmd */ | 49 /* KeyCode.KeyS */;

export function createEditor(opts: CreateEditorOptions): monaco.editor.IStandaloneCodeEditor {
  defineThemes();
  const editor = monaco.editor.create(opts.host, {
    value: opts.value,
    language: opts.language || "plaintext",
    theme: "zed-one-dark",
    automaticLayout: true,
    fontSize: 13,
    minimap: { enabled: true },
    fontFamily: "'Zed Mono', ui-monospace, SFMono-Regular, Menlo, monospace",
    scrollBeyondLastLine: false,
    smoothScrolling: true,
    inlineSuggest: { enabled: true },
  });
  if (opts.onChange) editor.onDidChangeModelContent(() => opts.onChange?.());
  if (opts.onSave) editor.addCommand(SAVE_KEY, () => opts.onSave?.());
  if (opts.onCursor) {
    editor.onDidChangeCursorPosition((e) => opts.onCursor?.(e.position.lineNumber, e.position.column));
  }
  return editor;
}

export interface InlineCompletionContextData {
  prefix: string;
  suffix: string;
  language: string;
}

export type InlineCompletionSource = (ctx: InlineCompletionContextData) => Promise<string | null>;

/** Ghost-text autocomplete (FIM) — kama Cursor Tab. */
export function registerInlineCompletions(
  editor: monaco.editor.IStandaloneCodeEditor,
  source: InlineCompletionSource,
): monaco.IDisposable {
  const provider: monaco.languages.InlineCompletionsProvider = {
    async provideInlineCompletions(model, position) {
      const lastLine = model.getLineCount();
      const offset = model.getOffsetAt(position);
      const prefix = model.getValue().slice(0, offset);
      const suffix = model.getValue().slice(offset);
      void lastLine;
      const text = await source({ prefix, suffix, language: model.getLanguageId() });
      if (!text) return { items: [] };
      return { items: [{ insertText: text }] };
    },
    freeInlineCompletions() {
      /* nothing retained */
    },
  };
  return monaco.languages.registerInlineCompletionsProvider({ pattern: "**" }, provider);
}

export type InlineEditRunner = (
  selection: string,
  language: string,
  apply: (code: string) => void,
) => void;

/** Cmd/Ctrl+K AI inline edit — kama Cursor quick-edit. */
export function addInlineEditAction(
  editor: monaco.editor.IStandaloneCodeEditor,
  run: InlineEditRunner,
): monaco.IDisposable {
  return editor.addAction({
    id: "mbide.inlineEdit",
    label: "AI Inline Edit",
    keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK],
    contextMenuGroupId: "1_modification",
    run: (ed) => {
      const model = ed.getModel();
      const sel = ed.getSelection();
      if (!model || !sel || sel.isEmpty()) return;
      const selection = model.getValueInRange(sel);
      run(selection, model.getLanguageId(), (code) => {
        ed.executeEdits("mbide.inlineEdit", [{ range: sel, text: code, forceMoveMarkers: true }]);
        ed.pushUndoStop();
      });
    },
  });
}

export { monaco };
