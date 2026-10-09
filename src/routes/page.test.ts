import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Page from "./+page.svelte";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("desktop workspace", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("renders named sections and accessible controls without starting media work", async () => {
    render(Page);
    expect(
      screen.getByRole("navigation", { name: "Main navigation" }),
    ).toBeTruthy();
    expect(screen.getByRole("main")).toBeTruthy();
    expect(screen.getByRole("region", { name: "Media preview" })).toBeTruthy();
    expect(screen.getByRole("region", { name: "Downloads" })).toBeTruthy();
    const input = screen.getByRole("textbox", { name: "Public media URL" });
    expect(input.getAttribute("aria-describedby")).toBe("url-help");
    await fireEvent.input(input, {
      target: { value: "https://media.example.org/watch" },
    });
    await fireEvent.keyDown(input, { key: "Enter" });
    const preview = screen.getByRole("button", {
      name: "Preview media",
    }) as HTMLButtonElement;
    expect(preview.disabled).toBe(false);
    expect(invoke).not.toHaveBeenCalled();
    expect(screen.queryByText(/Welcome to Tauri|Greet|Vite Logo/)).toBeNull();
  });

  it("navigates to empty downloads, moves focus and preserves the entered URL", async () => {
    render(Page);
    await fireEvent.input(screen.getByRole("textbox"), {
      target: { value: "https://media.example.org/watch" },
    });
    await fireEvent.click(
      screen.getByRole("button", { name: "View downloads" }),
    );
    const heading = await screen.findByRole("heading", {
      name: "Downloads",
      level: 1,
    });
    expect(document.activeElement).toBe(heading);
    expect(screen.getByText("Your queue is clear")).toBeTruthy();
    expect(
      screen
        .getByRole("button", { name: "Downloads" })
        .getAttribute("aria-current"),
    ).toBe("page");
    await fireEvent.click(
      screen.getByRole("button", { name: "Back to new media" }),
    );
    expect((screen.getByRole("textbox") as HTMLInputElement).value).toBe(
      "https://media.example.org/watch",
    );
    expect(invoke).not.toHaveBeenCalled();
  });

  it("checks dependencies only when settings opens and announces IPC errors", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("IPC unavailable"));
    render(Page);
    expect(invoke).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(invoke).toHaveBeenCalledExactlyOnceWith("check_dependencies");
    expect(document.activeElement).toBe(
      screen.getByRole("heading", { name: "Settings", level: 1 }),
    );
    expect(screen.getByText("Dark theme")).toBeTruthy();
  });
});
