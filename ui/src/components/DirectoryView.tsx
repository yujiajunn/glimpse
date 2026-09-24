import type { PreviewPayload } from "../types";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function DirectoryView({
  payload,
}: {
  payload: Extract<PreviewPayload, { kind: "directory" }>;
}) {
  return (
    <div className="text-body">
      <div style={{ color: "rgba(255,255,255,0.6)", marginBottom: 12 }}>
        共 {payload.total_count} 项
      </div>
      <div style={{ overflow: "auto", maxHeight: "70vh" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
          <thead>
            <tr style={{ color: "rgba(255,255,255,0.5)" }}>
              <th style={{ textAlign: "left", padding: 4 }}>名称</th>
              <th style={{ textAlign: "right", padding: 4, width: 100 }}>大小</th>
              <th style={{ textAlign: "right", padding: 4, width: 160 }}>修改时间</th>
            </tr>
          </thead>
          <tbody>
            {payload.entries.map((e, i) => (
              <tr key={i} style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }}>
                <td style={{ padding: 4 }}>
                  {e.is_dir ? "📁" : "📄"} {e.name}
                </td>
                <td style={{ padding: 4, textAlign: "right", color: "rgba(255,255,255,0.6)" }}>
                  {e.is_dir ? "—" : formatSize(e.size)}
                </td>
                <td style={{ padding: 4, textAlign: "right", color: "rgba(255,255,255,0.4)" }}>
                  {e.modified
                    ? new Date(e.modified * 1000).toLocaleString()
                    : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}