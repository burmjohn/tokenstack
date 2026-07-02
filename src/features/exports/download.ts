export function downloadTextFile(filename: string, text: string, type = "text/plain;charset=utf-8") {
  const blob = new Blob([text], { type });
  downloadBlob(filename, blob);
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
