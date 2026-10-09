import { invoke } from "@tauri-apps/api/core";

export interface MediaFormat {
  formatId: string;
  extension: string | null;
  formatNote: string | null;
  audioCodec: string | null;
  videoCodec: string | null;
  hasAudio: boolean | null;
  hasVideo: boolean | null;
  width: number | null;
  height: number | null;
  fps: number | null;
  audioBitrate: number | null;
  videoBitrate: number | null;
  totalBitrate: number | null;
  sampleRate: number | null;
  fileSize: number | null;
  fileSizeApprox: number | null;
}

export interface MediaInfo {
  id: string;
  title: string | null;
  /** Duration in seconds; null for unknown/live media. */
  duration: number | null;
  thumbnail: string | null;
  uploader: string | null;
  formats: MediaFormat[];
}

export type ExtractionErrorCode =
  | "invalid_url"
  | "missing_yt_dlp"
  | "invalid_configuration"
  | "permission_denied"
  | "timeout"
  | "output_limit"
  | "unsupported_content"
  | "authentication_required"
  | "runtime_required"
  | "network_error"
  | "extraction_failed"
  | "invalid_metadata";

export interface ExtractionError {
  code: ExtractionErrorCode;
  message: string;
}

/** Only metadata extraction is exposed; no executable path or flags cross IPC. */
export function getMediaInfo(url: string): Promise<MediaInfo> {
  return invoke<MediaInfo>("get_media_info", { url });
}
