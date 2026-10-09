import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import StatusView from "./DependencyStatus.svelte";
import type { DependencyStatus } from "./dependencies";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const statuses: DependencyStatus[] = [
  { tool: "yt-dlp", available: true, version: "2026.10.01", error: null },
  { tool: "ffmpeg", available: true, version: "8.0.1", error: null },
  { tool: "ffprobe", available: true, version: "8.0.1", error: null },
];

describe("dependency status", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("displays every tool, version and availability from typed IPC", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(statuses);
    render(StatusView);
    expect(await screen.findByRole("table")).toBeTruthy();
    for (const status of statuses) {
      expect(screen.getByRole("rowheader", { name: status.tool })).toBeTruthy();
    }
    expect(screen.getByText("2026.10.01")).toBeTruthy();
    expect(screen.getAllByText("8.0.1")).toHaveLength(2);
    expect(screen.getAllByText("Available")).toHaveLength(3);
    expect(invoke).toHaveBeenCalledExactlyOnceWith("check_dependencies");
  });

  it("shows actionable missing/unsupported errors and preserves detected versions", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([
      {
        ...statuses[0],
        available: false,
        version: null,
        error: {
          code: "not_found",
          message: "Install yt-dlp and add it to PATH.",
        },
      },
      statuses[1],
      {
        ...statuses[2],
        available: false,
        version: "5.1",
        error: {
          code: "unsupported_version",
          message: "Install ffprobe >= 6.0.",
        },
      },
    ]);
    render(StatusView);
    expect(
      await screen.findByText("Install yt-dlp and add it to PATH."),
    ).toBeTruthy();
    expect(screen.getByText("Not detected")).toBeTruthy();
    expect(screen.getByText("5.1")).toBeTruthy();
    expect(screen.getAllByText("Unavailable")).toHaveLength(2);
    expect(
      screen.getByRole("link", { name: "FFmpeg package" }).getAttribute("href"),
    ).toBe("https://ffmpeg.org/download.html");
  });

  it("handles IPC errors and allows retrying", async () => {
    vi.mocked(invoke)
      .mockRejectedValueOnce(new Error("IPC unavailable"))
      .mockResolvedValueOnce(statuses);
    render(StatusView);
    expect(await screen.findByRole("alert")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Check again" }));
    expect(await screen.findByRole("table")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("shows an explicit empty response and allows a manual recheck", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([]).mockResolvedValueOnce(statuses);
    render(StatusView);
    expect(
      await screen.findByText(
        "No tool information returned. Select Check again to retry.",
      ),
    ).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Check again" }));
    expect(await screen.findByRole("table")).toBeTruthy();
  });

  it("disables rechecks until the current request finishes and refreshes old results", async () => {
    let resolve!: (value: DependencyStatus[]) => void;
    vi.mocked(invoke).mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    render(StatusView);
    const button = screen.getByRole("button", {
      name: "Checking dependencies…",
    }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(screen.getByRole("status").textContent).toBe(
      "Checking local tools…",
    );
    await fireEvent.click(button);
    expect(invoke).toHaveBeenCalledTimes(1);
    resolve(statuses);
    await waitFor(() => expect(button.disabled).toBe(false));
    vi.mocked(invoke).mockResolvedValueOnce(
      statuses.map((status) => ({ ...status, version: "9.0" })),
    );
    await fireEvent.click(button);
    await waitFor(() => expect(screen.getAllByText("9.0")).toHaveLength(3));
    expect(screen.queryByText("2026.10.01")).toBeNull();
  });
});
