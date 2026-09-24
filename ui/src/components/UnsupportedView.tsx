import type { PreviewPayload } from "../types";

export function UnsupportedView({ payload }: { payload: Extract<PreviewPayload, { kind: "unsupported" }> }) {
  return (
    <div className="unsupported">
      <div style={{ fontSize: 48, marginBottom: 12 }}>∅</div>
      <div>无预览可用</div>
      <div style={{ fontSize: 11, marginTop: 8, opacity: 0.7 }}>{payload.reason}</div>
    </div>
  );
}