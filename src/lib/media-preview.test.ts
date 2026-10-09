import { invoke } from "@tauri-apps/api/core";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import fixture from "../../src-tauri/tests/fixtures/media.normalized.json";
import type { MediaInfo } from "./media-info";
import Page from "../routes/+page.svelte";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

async function submit(url: string) {
  const input = screen.getByRole("textbox", { name: "Public media URL" });
  await fireEvent.input(input, { target: { value: url } });
  await fireEvent.submit(input.closest("form")!);
}

function deferred() {
  let resolve!: (info: MediaInfo) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<MediaInfo>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

describe("media preview", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("analyzes a trimmed URL and displays the typed metadata and actual source streams", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(fixture);
    render(Page);
    await submit("  https://media.example.org/watch?v=sample&list=ignored  ");
    expect(
      await screen.findByRole("heading", { name: fixture.title }),
    ).toBeTruthy();
    expect(screen.getByText("Open film studio")).toBeTruthy();
    expect(screen.getByText("2:05")).toBeTruthy();
    const image = screen.getByRole("img") as HTMLImageElement;
    expect(image.src).toBe(fixture.thumbnail);
    expect(image.getAttribute("referrerpolicy")).toBe("no-referrer");
    const table = screen.getByRole("table", {
      name: "Available source formats",
    });
    expect(within(table).getByText("Audio only")).toBeTruthy();
    expect(within(table).getByText("Video only")).toBeTruthy();
    expect(within(table).getByText("Audio + video")).toBeTruthy();
    expect(within(table).getByText("M4A")).toBeTruthy();
    expect(within(table).getByText("1.9 MiB")).toBeTruthy();
    expect(within(table).getByText("~37.2 MiB")).toBeTruthy();
    expect(
      within(screen.getByRole("region", { name: "Media preview" })).queryByRole(
        "button",
        { name: /Download|Convert/ },
      ),
    ).toBeNull();
    expect(invoke).toHaveBeenCalledExactlyOnceWith("get_media_info", {
      url: "https://media.example.org/watch?v=sample&list=ignored",
    });
  });

  it.each([
    "",
    "not a URL",
    "file:///C:/secret",
    "ftp://media.example.org/file",
    "https:///media.example.org",
    "https://user:secret@media.example.org",
    "https://media.example.org/a b",
    "https://media.example.org/\u0007",
    "https://media.example.org/" + "é".repeat(2100),
  ])("rejects invalid input %j before IPC", async (url) => {
    render(Page);
    await submit(url);
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByRole("textbox").getAttribute("aria-invalid")).toBe(
      "true",
    );
    expect(screen.getByRole("textbox").getAttribute("aria-errormessage")).toBe(
      "preview-error",
    );
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([
    ["invalid_url", "Check the media URL"],
    ["missing_yt_dlp", "yt-dlp is missing"],
    ["timeout", "Analysis timed out"],
    ["unsupported_content", "Media unavailable"],
    ["extraction_failed", "Could not analyze this media"],
    ["runtime_required", "Additional runtime required"],
    ["authentication_required", "This media requires access"],
    ["network_error", "Could not reach the media"],
    ["invalid_metadata", "Unexpected media metadata"],
    ["invalid_configuration", "Check your yt-dlp setup"],
    ["permission_denied", "yt-dlp cannot run"],
    ["output_limit", "Too much metadata returned"],
  ])(
    "renders a helpful %s state without native diagnostics",
    async (code, title) => {
      vi.mocked(invoke).mockRejectedValueOnce({
        code,
        message: "ERROR: process log token=private-secret",
      });
      render(Page);
      await submit("https://media.example.org/watch");
      expect(await screen.findByRole("heading", { name: title })).toBeTruthy();
      expect(screen.getByRole("alert")).toBeTruthy();
      expect(screen.queryByText(/private-secret|process log/)).toBeNull();
      expect(
        (
          screen.getByRole("button", {
            name: "Preview media",
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(false);
    },
  );

  it("handles an unknown IPC failure safely and retries with the same URL", async () => {
    vi.mocked(invoke)
      .mockRejectedValueOnce("raw native log secret")
      .mockResolvedValueOnce(fixture);
    render(Page);
    await submit("https://media.example.org/watch");
    expect(
      await screen.findByText("Desktop connection unavailable"),
    ).toBeTruthy();
    expect(screen.queryByText(/raw native log/)).toBeNull();
    await fireEvent.submit(screen.getByRole("textbox").closest("form")!);
    expect(
      await screen.findByRole("heading", { name: fixture.title }),
    ).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("renders unknown optional fields and an empty formats list without inventing media", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      ...fixture,
      title: null,
      duration: null,
      uploader: null,
      thumbnail: null,
      formats: [],
    });
    render(Page);
    await submit("https://media.example.org/live");
    expect(
      await screen.findByRole("heading", { name: "Untitled media" }),
    ).toBeTruthy();
    for (const message of [
      "Duration unknown",
      "Uploader unknown",
      "Thumbnail unavailable",
      "No format information is available for this media.",
    ])
      expect(screen.getByText(message)).toBeTruthy();
    expect(screen.queryByRole("img")).toBeNull();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("keeps zero duration and unknown streams explicit", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      ...fixture,
      duration: 0,
      formats: [
        {
          ...fixture.formats[0],
          extension: null,
          audioCodec: null,
          videoCodec: null,
          hasAudio: null,
          hasVideo: null,
          fileSize: 0,
        },
      ],
    });
    render(Page);
    await submit("https://media.example.org/watch");
    expect(await screen.findByText("0:00")).toBeTruthy();
    expect(screen.getByText("Streams unknown")).toBeTruthy();
    expect(screen.getByText("Container unknown")).toBeTruthy();
    expect(screen.getByText("0 B")).toBeTruthy();
  });

  it("falls back when a thumbnail fails and resets the image state on another result", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(fixture)
      .mockResolvedValueOnce({
        ...fixture,
        id: "next",
        title: "Next media",
        thumbnail: "https://media.example.org/next.jpg",
      });
    render(Page);
    await submit("https://media.example.org/first");
    const image = await screen.findByRole("img");
    await fireEvent.error(image);
    expect(screen.getByText("Thumbnail unavailable")).toBeTruthy();
    expect(screen.getByRole("heading", { name: fixture.title })).toBeTruthy();
    await submit("https://media.example.org/next");
    expect(((await screen.findByRole("img")) as HTMLImageElement).src).toBe(
      "https://media.example.org/next.jpg",
    );
  });

  it("does not load an unsafe thumbnail URL or interpret metadata as HTML", async () => {
    const title = '<img src=x onerror="alert(1)">';
    vi.mocked(invoke).mockResolvedValueOnce({
      ...fixture,
      title,
      thumbnail: "javascript:alert(1)",
    });
    render(Page);
    await submit("https://media.example.org/watch");
    expect(await screen.findByRole("heading", { name: title })).toBeTruthy();
    expect(screen.queryByRole("img")).toBeNull();
  });

  it("allows editing during loading, prevents concurrent requests and ignores a late success", async () => {
    const first = deferred();
    vi.mocked(invoke)
      .mockReturnValueOnce(first.promise)
      .mockResolvedValueOnce({ ...fixture, title: "Latest media" });
    render(Page);
    await submit("https://media.example.org/first");
    expect(screen.getByRole("status").textContent).toContain(
      "Analyzing media…",
    );
    expect(
      (screen.getByRole("button", { name: "Analyzing…" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
    await fireEvent.submit(screen.getByRole("textbox").closest("form")!);
    expect(invoke).toHaveBeenCalledTimes(1);
    await fireEvent.input(screen.getByRole("textbox"), {
      target: { value: "https://media.example.org/latest" },
    });
    expect(screen.getByText("URL changed during analysis")).toBeTruthy();
    first.resolve(fixture);
    await waitFor(() => expect(screen.queryByRole("status")).toBeNull());
    expect(screen.queryByRole("heading", { name: fixture.title })).toBeNull();
    await fireEvent.submit(screen.getByRole("textbox").closest("form")!);
    expect(
      await screen.findByRole("heading", { name: "Latest media" }),
    ).toBeTruthy();
    expect(invoke).toHaveBeenLastCalledWith("get_media_info", {
      url: "https://media.example.org/latest",
    });
  });

  it("invalidates late errors even if the user returns to the original URL", async () => {
    const request = deferred();
    vi.mocked(invoke).mockReturnValueOnce(request.promise);
    render(Page);
    await submit("https://media.example.org/first");
    for (const url of [
      "https://media.example.org/second",
      "https://media.example.org/first",
    ])
      await fireEvent.input(screen.getByRole("textbox"), {
        target: { value: url },
      });
    request.reject({ code: "timeout", message: "old failure" });
    await waitFor(() => expect(screen.queryByRole("status")).toBeNull());
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("clears a successful preview as soon as the URL changes", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(fixture);
    render(Page);
    await submit("https://media.example.org/first");
    expect(
      await screen.findByRole("heading", { name: fixture.title }),
    ).toBeTruthy();
    await fireEvent.input(screen.getByRole("textbox"), {
      target: { value: "https://media.example.org/second" },
    });
    expect(screen.queryByRole("heading", { name: fixture.title })).toBeNull();
    expect(screen.queryByRole("table")).toBeNull();
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("keeps navigation responsive while loading and retains a current result", async () => {
    const request = deferred();
    vi.mocked(invoke).mockReturnValueOnce(request.promise);
    render(Page);
    await submit("https://media.example.org/watch");
    await fireEvent.click(
      screen.getByRole("button", { name: "View downloads" }),
    );
    expect(screen.getByText("Your queue is clear")).toBeTruthy();
    request.resolve(fixture);
    await fireEvent.click(
      screen.getByRole("button", { name: "Back to new media" }),
    );
    expect(
      await screen.findByRole("heading", { name: fixture.title }),
    ).toBeTruthy();
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});
