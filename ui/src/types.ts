// 与 crates/preview-protocol + src-tauri/src/preview/mod.rs 保持一致

export type OfficeKind = "docx" | "xlsx" | "pptx";

export type OfficeContent =
  | { format: "html"; html: string }
  | { format: "sheets"; sheets: SheetData[] }
  | { format: "slides"; slides: SlideData[] };

export interface SheetData {
  name: string;
  rows: string[][];
}

export interface SlideData {
  index: number;
  title: string;
  text: string;
}

export interface ArchiveEntry {
  name: string;
  size: number;
  is_dir: boolean;
  modified: number | null;
}

export interface DirEntry {
  name: string;
  is_dir: boolean;
  size: number;
  modified: number | null;
}

export type PreviewPayload =
  | {
      kind: "image";
      data_base64: string;
      width: number;
      height: number;
      mime: string;
    }
  | {
      kind: "pdf";
      data_base64: string;
      page_count: number;
    }
  | {
      kind: "video";
      url: string;
      duration_sec: number;
      width: number;
      height: number;
    }
  | {
      kind: "audio";
      url: string;
      duration_sec: number;
    }
  | {
      kind: "text";
      content: string;
      is_markdown: boolean;
    }
  | {
      kind: "office";
      office_kind: OfficeKind;
      content: OfficeContent;
      title: string;
    }
  | {
      kind: "font";
      family: string;
      style: string;
      glyph_count: number;
      specimen_png: string;
    }
  | {
      kind: "archive";
      entries: ArchiveEntry[];
      total_size: number;
    }
  | {
      kind: "directory";
      entries: DirEntry[];
      total_count: number;
    }
  | {
      kind: "unsupported";
      reason: string;
    };

export interface PreviewRequest {
  path: string;
  max_width: number;
  max_height: number;
}