import type { PreviewPayload } from "../types";

export function ImageView({ payload }: { payload: Extract<PreviewPayload, { kind: "image" }> }) {
  const src = `data:${payload.mime};base64,${payload.data_base64}`;
  return <img className="preview-image" src={src} alt="" draggable={false} />;
}