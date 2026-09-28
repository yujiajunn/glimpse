import { useEffect, useRef, useState } from "react";
import type { PreviewPayload } from "../types";

const SAMPLE_TEXT = "AaBbCc 123 一二三 中英文";

export function FontView({
  payload,
  fontPath,
}: {
  payload: Extract<PreviewPayload, { kind: "font" }>;
  fontPath?: string;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [fontReady, setFontReady] = useState(false);
  const [fontFamily] = useState(() => `glimpse-font-${Math.random().toString(36).slice(2)}`);

  // 用 FontFace API 加载字体二进制
  useEffect(() => {
    if (typeof FontFace === "undefined") {
      return;
    }

    try {
      const bytes = Uint8Array.from(atob(payload.font_data_base64), (c) => c.charCodeAt(0));
      const face = new FontFace(fontFamily, bytes);
      face
        .load()
        .then((loaded) => {
          document.fonts.add(loaded);
          setFontReady(true);
        })
        .catch(() => {
          // fallback to system font
        });
    } catch {
      // ignore
    }
  }, [payload.font_data_base64, fontFamily]);

  // 字体加载完成 → 在 canvas 上画字符表
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = canvas.clientWidth * dpr;
    canvas.height = canvas.clientHeight * dpr;
    ctx.scale(dpr, dpr);

    const w = canvas.clientWidth;
    const h = canvas.clientHeight;

    // 背景
    ctx.fillStyle = "rgba(0,0,0,0.3)";
    ctx.fillRect(0, 0, w, h);

    // 大字号演示
    ctx.fillStyle = "#f5f5f7";
    ctx.font = fontReady
      ? `bold 48px "${fontFamily}", system-ui`
      : "bold 48px system-ui";
    ctx.fillText(SAMPLE_TEXT, 24, 70);

    // 元数据
    ctx.fillStyle = "rgba(255,255,255,0.55)";
    ctx.font = "12px system-ui";
    ctx.fillText(`Family: ${payload.family}`, 24, 100);
    ctx.fillText(`Style: ${payload.style}`, 24, 118);
    ctx.fillText(`Glyphs: ${payload.glyph_count}`, 24, 136);
    if (fontPath) ctx.fillText(`Path: ${fontPath}`, 24, 154);

    // 多尺寸演示
    const sizes = [12, 16, 20, 24, 32, 48];
    ctx.fillStyle = "#f5f5f7";
    sizes.forEach((size, i) => {
      ctx.font = fontReady
        ? `${size}px "${fontFamily}", system-ui`
        : `${size}px system-ui`;
      ctx.fillText(`${size}px  The quick brown fox`, 24, 190 + i * (size + 8));
    });
  }, [fontReady, payload, fontPath, fontFamily]);

  return (
    <div style={{ padding: 24, width: "100%", height: "100%" }}>
      <canvas
        ref={canvasRef}
        style={{ width: "100%", height: "100%", display: "block" }}
      />
    </div>
  );
}