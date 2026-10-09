import { invoke } from "@tauri-apps/api/core";

export type Tool = "yt-dlp" | "ffmpeg" | "ffprobe";
export type DependencyErrorCode =
  | "not_found"
  | "permission_denied"
  | "spawn_failed"
  | "timeout"
  | "output_limit"
  | "non_zero_exit"
  | "invalid_output"
  | "unsupported_version"
  | "invalid_configuration";

export interface DependencyStatus {
  tool: Tool;
  available: boolean;
  version: string | null;
  error: { code: DependencyErrorCode; message: string } | null;
}

export function checkDependencies(): Promise<DependencyStatus[]> {
  return invoke<DependencyStatus[]>("check_dependencies");
}
