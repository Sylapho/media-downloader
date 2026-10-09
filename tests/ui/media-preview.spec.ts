import { expect, test } from "@playwright/test";
import mediaFixture from "../../src-tauri/tests/fixtures/media.normalized.json" with { type: "json" };

test("Enter validates input, loading stays editable, and late metadata cannot overwrite a new URL", async ({
  page,
}) => {
  await page.addInitScript((media) => {
    const urls: string[] = [];
    let finish: ((value: unknown) => void) | undefined;
    Object.defineProperty(window, "__previewRequests", { value: urls });
    Object.defineProperty(window, "__finishPreview", {
      value: () =>
        finish?.({ ...media, title: "Obsolete media", thumbnail: null }),
    });
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {
        invoke: async (command: string, args: { url: string }) => {
          if (command !== "get_media_info")
            throw new Error("Unexpected IPC command");
          urls.push(args.url);
          if (urls.length === 1)
            return new Promise((resolve) => {
              finish = resolve;
            });
          return { ...media, title: "Latest media", thumbnail: null };
        },
      },
    });
  }, mediaFixture);
  await page.goto("/");
  const input = page.getByRole("textbox", { name: "Public media URL" });
  await input.fill("not a URL");
  await input.press("Enter");
  await expect(page.getByRole("alert")).toContainText("Check the media URL");
  expect(
    await page.evaluate(() => Reflect.get(window, "__previewRequests")),
  ).toEqual([]);

  await input.fill("https://media.example.org/first");
  await input.press("Enter");
  await expect(page.getByRole("status")).toContainText("Analyzing media…");
  await expect(page.getByRole("button", { name: "Analyzing…" })).toBeDisabled();
  await input.press("Enter");
  await input.fill("https://media.example.org/latest");
  await expect(page.getByRole("status")).toContainText(
    "URL changed during analysis",
  );
  expect(
    await page.evaluate(() => Reflect.get(window, "__previewRequests")),
  ).toEqual(["https://media.example.org/first"]);
  await page.evaluate(() => Reflect.get(window, "__finishPreview")());
  await expect(
    page.getByRole("button", { name: "Preview media" }),
  ).toBeEnabled();
  await expect(
    page.getByRole("heading", { name: "Obsolete media" }),
  ).toHaveCount(0);
  await input.press("Enter");
  await expect(
    page.getByRole("heading", { name: "Latest media" }),
  ).toBeVisible();
  expect(
    await page.evaluate(() => Reflect.get(window, "__previewRequests")),
  ).toEqual([
    "https://media.example.org/first",
    "https://media.example.org/latest",
  ]);
  await expect(page.getByText("Video only", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("button", { name: /^Download |Convert/ }),
  ).toHaveCount(0);
});

test("missing dependency feedback guides setup and never exposes a process log", async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {
        invoke: async (command: string) => {
          if (command === "get_media_info")
            throw {
              code: "missing_yt_dlp",
              message: "ERROR native log secret token",
            };
          return [];
        },
      },
    });
  });
  await page.goto("/");
  await page.getByRole("textbox").fill("https://media.example.org/watch");
  await page.getByRole("button", { name: "Preview media" }).click();
  await expect(page.getByRole("alert")).toContainText("yt-dlp is missing");
  await expect(page.getByText(/native log secret/)).toHaveCount(0);
  await page.getByRole("button", { name: "Open settings" }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", level: 1 }),
  ).toBeFocused();
});
