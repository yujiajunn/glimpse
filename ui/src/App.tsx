import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { PreviewPayload } from "./types";
import { ImageView } from "./components/ImageView";
import { PdfView } from "./components/PdfView";
import { VideoView } from "./components/VideoView";
import { TextView } from "./components/TextView";
import { UnsupportedView } from "./components/UnsupportedView";
import { OfficeView } from "./components/OfficeView";
import { FontView } from "./components/FontView";
import { ArchiveView } from "./components/ArchiveView";
import { DirectoryView } from "./components/DirectoryView";

interface PreviewState {
  filePath: string;
  payload: PreviewPayload | null;
  loading: boolean;
  error: string | null;
}

export function App() {
  const [state, setState] = useState<PreviewState>({
    filePath: "",
    payload: null,
    loading: false,
    error: null,
  });

  const loadPreview = useCallback(async (path: string) => {
    setState((s) => ({ ...s, filePath: path, loading: true, error: null }));
    try {
      const payload = await invoke<PreviewPayload>("cmd_show_preview", { path });
      setState((s) => ({ ...s, payload, loading: false }));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setState((s) => ({ ...s, error: msg, loading: false }));
    }
  }, []);

  // 监听 shell 触发（命名管道 → main → window.emit → 我们）
  useEffect(() => {
    const win = getCurrentWindow();
    const unlisten = win.listen<string>("shell-trigger", (event) => {
      loadPreview(event.payload);
    });
    return () => {
      unlisten.then((u) => u());
    };
  }, [loadPreview]);

  // ESC 关闭预览
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        invoke("cmd_hide_preview");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const filename = state.filePath.split(/[\\/]/).pop() ?? "";

  return (
    <div className="app">
      <div className="titlebar">
        <span className="filename">{filename || "Glimpse"}</span>
        <div className="actions">
          <button
            onClick={() => invoke("cmd_hide_preview")}
            title="关闭 (Esc)"
          >
            ×
          </button>
        </div>
      </div>

      <div className="content">
        {state.loading && <div className="unsupported">加载中…</div>}
        {state.error && <div className="unsupported">错误: {state.error}</div>}
        {state.payload && (
          <PreviewRouter payload={state.payload} filePath={state.filePath} />
        )}
      </div>
    </div>
  );
}

function PreviewRouter({
  payload,
  filePath,
}: {
  payload: PreviewPayload;
  filePath: string;
}) {
  switch (payload.kind) {
    case "image":
      return <ImageView payload={payload} />;
    case "pdf":
      return <PdfView payload={payload} />;
    case "video":
    case "audio":
      return <VideoView payload={payload} />;
    case "text":
      return <TextView payload={payload} />;
    case "office":
      return <OfficeView payload={payload} />;
    case "font":
      return <FontView payload={payload} fontPath={filePath} />;
    case "archive":
      return <ArchiveView payload={payload} />;
    case "directory":
      return <DirectoryView payload={payload} />;
    case "unsupported":
      return <UnsupportedView payload={payload} />;
  }
}