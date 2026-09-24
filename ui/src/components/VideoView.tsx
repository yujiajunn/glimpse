import type { PreviewPayload } from "../types";

export function VideoView({ payload }: { payload: Extract<PreviewPayload, { kind: "video" | "audio" }> }) {
  if (payload.kind === "audio") {
    return (
      <div style={{ textAlign: "center", color: "rgba(255,255,255,0.7)" }}>
        <div style={{ fontSize: 48 }}>♪</div>
        <audio controls src={payload.url} autoPlay={false} />
      </div>
    );
  }
  return (
    <video
      className="preview-video"
      controls
      autoPlay
      preload="metadata"
      src={payload.url}
    />
  );
}