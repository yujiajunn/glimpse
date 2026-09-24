import type { PreviewPayload } from "../types";

export function OfficeView({
  payload,
}: {
  payload: Extract<PreviewPayload, { kind: "office" }>;
}) {
  const { content, title } = payload;

  if (content.format === "html") {
    return (
      <div
        className="text-body docx-body"
        dangerouslySetInnerHTML={{ __html: content.html }}
      />
    );
  }

  if (content.format === "sheets") {
    return (
      <div className="text-body">
        <h3 style={{ marginTop: 0 }}>{title}</h3>
        {content.sheets.map((sheet) => (
          <div key={sheet.name} style={{ marginBottom: 16 }}>
            <h4 style={{ color: "rgba(255,255,255,0.6)" }}>{sheet.name}</h4>
            <div style={{ overflow: "auto", maxHeight: "60vh" }}>
              <table style={{ borderCollapse: "collapse", fontSize: 12 }}>
                <tbody>
                  {sheet.rows.map((row, i) => (
                    <tr key={i}>
                      {row.map((cell, j) => (
                        <td
                          key={j}
                          style={{
                            border: "1px solid rgba(255,255,255,0.1)",
                            padding: "4px 8px",
                            maxWidth: 240,
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          }}
                        >
                          {cell}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        ))}
      </div>
    );
  }

  // slides
  return (
    <div className="text-body">
      <h3 style={{ marginTop: 0 }}>{title}</h3>
      {content.slides.map((s) => (
        <div
          key={s.index}
          style={{
            marginBottom: 16,
            padding: 12,
            background: "rgba(255,255,255,0.04)",
            borderRadius: 6,
          }}
        >
          <div style={{ color: "rgba(255,255,255,0.5)", fontSize: 11 }}>
            Slide {s.index}
          </div>
          <h4>{s.title}</h4>
          <pre style={{ whiteSpace: "pre-wrap", fontSize: 12 }}>{s.text}</pre>
        </div>
      ))}
    </div>
  );
}