import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, it, vi } from "vitest";
import normalizedFixture from "../../src-tauri/tests/fixtures/media.normalized.json";
import {
  getMediaInfo,
  type MediaInfo,
  type ExtractionError,
} from "./media-info";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

it("accepts the same normalized contract checked by Rust and passes only the URL", async () => {
  const fixture: MediaInfo = normalizedFixture;
  vi.mocked(invoke).mockResolvedValueOnce(fixture);
  const url = "https://media.example.org/watch?v=sample&list=ignored";
  expect(await getMediaInfo(url)).toEqual(fixture);
  expect(invoke).toHaveBeenCalledExactlyOnceWith("get_media_info", { url });
});

it.each([
  "invalid_url",
  "unsupported_content",
  "timeout",
  "missing_yt_dlp",
] as const)("preserves the typed %s error from Rust", async (code) => {
  const error: ExtractionError = { code, message: "Actionable explanation" };
  vi.mocked(invoke).mockRejectedValueOnce(error);
  await expect(getMediaInfo("https://media.example.org/watch")).rejects.toEqual(
    error,
  );
});
