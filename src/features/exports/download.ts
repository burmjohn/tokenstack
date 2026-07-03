import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "../../lib/api/tauri";

export type TextDownloadResult =
  | { status: "downloaded" }
  | { status: "saved"; path: string }
  | { status: "failed"; error: string };

export async function downloadTextFile(filename: string, text: string, type = "text/plain;charset=utf-8"): Promise<TextDownloadResult> {
  if (isTauriRuntime()) {
    try {
      const path = await invoke<string>("save_text_export", { filename, contents: text });
      return { status: "saved", path };
    } catch (error) {
      return { status: "failed", error: error instanceof Error ? error.message : String(error) };
    }
  }

  const blob = new Blob([text], { type });
  downloadBlob(filename, blob);
  return { status: "downloaded" };
}

export async function downloadCanvasPng(filename: string, canvas: HTMLCanvasElement): Promise<boolean> {
  const blob = await new Promise<Blob | null>((resolve) => {
    canvas.toBlob(resolve, "image/png");
  });

  if (!blob) {
    return false;
  }

  downloadBlob(filename, blob);
  return true;
}

function downloadBlob(filename: string, blob: Blob) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.rel = "noopener";
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}
