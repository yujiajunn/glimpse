import { useEffect, useRef } from "react";
import type { PreviewPayload } from "../types";

export function FontView({
  payload,
  fontPath,
}: {
  payload: Extract<PreviewPayload, { kind: "font" }>;
  fontPath?: string;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    // @ts-expect-error - 浏览器原生 FontFace API
    if (typeof FontFace === "undefined") {
      return;
    }

    // 这里需要 Tauri 给我们字体二进制 → 暂时显示元数据
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    canvas.width = canvas.clientWidth * 2;
    canvas.height = canvas.clientHeight * 2;
    ctx.scale(2, 2);

    ctx.fillStyle = "rgba(255,255,255,0.85)";
    ctx.font = "48px system-ui";
    ctx.fillText(payload.family, 20, 60);

    ctx.fillStyle = "rgba(255,255,255,0.5)";
    ctx.font = "14px system-ui";
    ctx.fillText(`Style: ${payload.style}`, 20, 90);
    ctx.fillText(`Glyphs: ${payload.glyph_count}`, 20, 110);
    ctx.fillText("(实时字体预览需加载字体二进制，未实现)", 20, 140);
  }, [payload]);

  return (
    <div style={{ padding: 24, width: "100%" }}>
      <canvas
        ref={canvasRef}
        style={{ width: "100%", height: 240, display: "block" }}
      />
    </div>
  );
}