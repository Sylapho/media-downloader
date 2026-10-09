import type { MediaFormat } from "./media-info";

export function durationLabel(seconds: number | null): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0)
    return "Duration unknown";
  const whole = Math.floor(seconds);
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor((whole % 3600) / 60);
  const remainder = String(whole % 60).padStart(2, "0");
  return hours
    ? `${hours}:${String(minutes).padStart(2, "0")}:${remainder}`
    : `${minutes}:${remainder}`;
}

export function streamLabel(format: MediaFormat): string {
  if (format.hasAudio === true && format.hasVideo === true)
    return "Audio + video";
  if (format.hasAudio === true && format.hasVideo === false)
    return "Audio only";
  if (format.hasAudio === false && format.hasVideo === true)
    return "Video only";
  if (format.hasAudio === true) return "Audio · video unknown";
  if (format.hasVideo === true) return "Video · audio unknown";
  if (format.hasAudio === false && format.hasVideo === false)
    return "No audio/video streams";
  if (format.hasAudio === false) return "Video unknown · no audio";
  if (format.hasVideo === false) return "Audio unknown · no video";
  return "Streams unknown";
}

export function formatDetails(format: MediaFormat): string {
  const details: string[] = [];
  if (format.width != null && format.height != null)
    details.push(`${format.width} × ${format.height}`);
  else if (format.height != null) details.push(`${format.height}p`);
  if (format.fps != null) details.push(`${format.fps} fps`);
  if (format.sampleRate != null) details.push(`${format.sampleRate} Hz`);
  if (format.totalBitrate != null) details.push(`${format.totalBitrate} kbps`);
  else if (format.audioBitrate != null)
    details.push(`Audio ${format.audioBitrate} kbps`);
  else if (format.videoBitrate != null)
    details.push(`Video ${format.videoBitrate} kbps`);
  return details.join(" · ") || "Quality unknown";
}

export function sizeLabel(format: MediaFormat): string {
  const bytes = format.fileSize ?? format.fileSizeApprox;
  if (bytes == null) return "Size unknown";
  const unit =
    bytes >= 1024 ** 3 ? 3 : bytes >= 1024 ** 2 ? 2 : bytes >= 1024 ? 1 : 0;
  const amount = new Intl.NumberFormat("en", {
    maximumFractionDigits: 1,
  }).format(bytes / 1024 ** unit);
  return `${format.fileSize == null ? "~" : ""}${amount} ${["B", "KiB", "MiB", "GiB"][unit]}`;
}

export function thumbnailUrl(value: string | null): string | null {
  if (!value) return null;
  try {
    const url = new URL(value);
    return ["http:", "https:"].includes(url.protocol) &&
      !url.username &&
      !url.password
      ? url.href
      : null;
  } catch {
    return null;
  }
}
