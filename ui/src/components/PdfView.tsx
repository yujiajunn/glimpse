import { useEffect, useRef, useState } from "react";
import type { PreviewPayload } from "../types";
import * as pdfjsLib from "pdfjs-dist";
import pdfWorker from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorker;

export function PdfView({ payload }: { payload: Extract<PreviewPayload, { kind: "pdf" }> }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [page, setPage] = useState(1);

  useEffect(() => {
    const bytes = Uint8Array.from(atob(payload.data_base64), (c) => c.charCodeAt(0));
    const task = pdfjsLib.getDocument({ data: bytes });

    task.promise.then((pdf) => {
      pdf.getPage(page).then((p) => {
        const container = containerRef.current;
        if (!container) return;
        container.innerHTML = "";

        const viewport = p.getViewport({ scale: 1.4 });
        const canvas = document.createElement("canvas");
        canvas.width = viewport.width;
        canvas.height = viewport.height;
        container.appendChild(canvas);

        p.render({ canvasContext: canvas.getContext("2d")!, viewport });
      });
    });
  }, [payload.data_base64, page]);

  return (
    <div style={{ width: "100%", height: "100%", overflow: "auto", display: "flex", flexDirection: "column", alignItems: "center" }}>
      <div style={{ padding: "8px", color: "rgba(255,255,255,0.6)" }}>
        第 {page} 页 / 共 {payload.page_count} 页
      </div>
      <div ref={containerRef} />
      <div style={{ display: "flex", gap: "8px", padding: "8px" }}>
        <button disabled={page <= 1} onClick={() => setPage((p) => p - 1)}>‹ 上一页</button>
        <button disabled={page >= payload.page_count} onClick={() => setPage((p) => p + 1)}>下一页 ›</button>
      </div>
    </div>
  );
}