import ReactMarkdown from "react-markdown";
import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { oneDark } from "@codemirror/theme-one-dark";
import type { PreviewPayload } from "../types";

export function TextView({ payload }: { payload: Extract<PreviewPayload, { kind: "text" }> }) {
  if (payload.is_markdown) {
    return (
      <div className="markdown-body">
        <ReactMarkdown>{payload.content}</ReactMarkdown>
      </div>
    );
  }
  return (
    <div className="text-body">
      <CodeMirror
        value={payload.content}
        editable={false}
        basicSetup={{ lineNumbers: false, foldGutter: false }}
        theme={oneDark}
        extensions={[markdown()]}
      />
    </div>
  );
}