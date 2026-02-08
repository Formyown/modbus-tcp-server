import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import "monaco-editor/esm/vs/basic-languages/javascript/javascript.contribution";
import "monaco-editor/esm/vs/language/typescript/monaco.contribution";
import "monaco-editor/esm/vs/editor/contrib/parameterHints/browser/parameterHints";
import "monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetController2";
import "monaco-editor/esm/vs/editor/contrib/suggest/browser/suggestController";

type MonacoEnv = {
  getWorker: (workerId: string, label: string) => Worker;
};

const globalScope = self as unknown as { MonacoEnvironment?: MonacoEnv };

globalScope.MonacoEnvironment = {
  getWorker(_workerId, label) {
    if (label === "typescript" || label === "javascript") {
      return new tsWorker();
    }
    return new editorWorker();
  },
};

const helperLib = `
type DataArea = "coils" | "discrete" | "input" | "holding";
type UpdatePayload = { area: DataArea; offset: number; values: number[] };

declare function writeCoils(
  offset: number,
  valueOrValues: boolean | number | Array<boolean | number>
): Promise<void>;
declare function writeDiscreteInputs(
  offset: number,
  valueOrValues: boolean | number | Array<boolean | number>
): Promise<void>;
declare function writeHoldingRegs(offset: number, valueOrValues: number | number[]): Promise<void>;
declare function writeInputRegs(offset: number, valueOrValues: number | number[]): Promise<void>;
declare function onChange(handler: (payload: UpdatePayload) => void): () => void;
declare function log(...args: unknown[]): void;
declare function setTimeout(
  handler: (...args: unknown[]) => void,
  ms?: number,
  ...args: unknown[]
): number;
declare function setInterval(
  handler: (...args: unknown[]) => void,
  ms?: number,
  ...args: unknown[]
): number;
declare function clearTimeout(id: number): void;
declare function clearInterval(id: number): void;
declare function sleep(ms: number): Promise<void>;
`;

const tsDefaults = monaco.languages.typescript?.javascriptDefaults ?? null;

if (tsDefaults) {
  tsDefaults.setCompilerOptions({
    allowNonTsExtensions: true,
    target: monaco.languages.typescript.ScriptTarget.ES2020,
    lib: ["es2020"],
  });

  tsDefaults.addExtraLib(helperLib, "inmemory://model/modbus-script-helpers.d.ts");
}

monaco.languages.registerCompletionItemProvider("javascript", {
  provideCompletionItems() {
    const range = undefined;
    return {
      suggestions: [
        {
          label: "writeCoils",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "writeCoils(${1:offset}, ${2:valueOrValues})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Write coils",
          documentation: "Write single or multiple coils.",
          range,
        },
        {
          label: "writeDiscreteInputs",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "writeDiscreteInputs(${1:offset}, ${2:valueOrValues})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Write discrete inputs",
          documentation: "Write single or multiple discrete inputs (DI).",
          range,
        },
        {
          label: "writeHoldingRegs",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "writeHoldingRegs(${1:offset}, ${2:valueOrValues})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Write holding registers",
          documentation: "Write single or multiple holding registers.",
          range,
        },
        {
          label: "writeInputRegs",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "writeInputRegs(${1:offset}, ${2:valueOrValues})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Write input registers",
          documentation: "Write single or multiple input registers.",
          range,
        },
        {
          label: "onChange",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "onChange(({ area, offset, values }) => {\n  $0\n})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Subscribe to changes",
          documentation: "Listen for any data changes; returns unlisten function.",
          range,
        },
        {
          label: "log",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "log(${1:message})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Log to console",
          documentation: "Append a log line to the logs panel.",
          range,
        },
        {
          label: "setTimeout",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "setTimeout(() => {\n  $0\n}, ${1:ms})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Delay execution",
          documentation: "Run a callback after a delay.",
          range,
        },
        {
          label: "setInterval",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "setInterval(() => {\n  $0\n}, ${1:ms})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Run on interval",
          documentation: "Run a callback repeatedly on an interval.",
          range,
        },
        {
          label: "clearTimeout",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "clearTimeout(${1:id})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Clear timeout",
          documentation: "Cancel a scheduled timeout.",
          range,
        },
        {
          label: "clearInterval",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "clearInterval(${1:id})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Clear interval",
          documentation: "Cancel a scheduled interval.",
          range,
        },
        {
          label: "sleep",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: "sleep(${1:ms})",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Delay",
          documentation: "Delay for a number of milliseconds.",
          range,
        },
      ],
    };
  },
});

export { monaco };
