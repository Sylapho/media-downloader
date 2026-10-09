import { expect, test, type Page } from "@playwright/test";
import mediaFixture from "../../src-tauri/tests/fixtures/media.normalized.json" with { type: "json" };

async function assertNoHorizontalClipping(page: Page) {
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  for (const element of await page.locator("button, input, table").all()) {
    const box = await element.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(
      page.viewportSize()!.width + 1,
    );
  }
}

for (const viewport of [
  { width: 800, height: 600 },
  { width: 640, height: 480 },
  { width: 1024, height: 768 },
  { width: 480, height: 640 },
  { width: 320, height: 640 },
  { width: 400, height: 300 },
]) {
  test(`workspace and settings fit ${viewport.width}×${viewport.height}`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.addInitScript((media) => {
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: {
          invoke: async (command: string) =>
            command === "get_media_info"
              ? media
              : [
                  {
                    tool: "yt-dlp",
                    available: true,
                    version: "2026.08.19",
                    error: null,
                  },
                  {
                    tool: "ffmpeg",
                    available: true,
                    version: "8.0.1-essentials_build-www.gyan.dev",
                    error: null,
                  },
                  {
                    tool: "ffprobe",
                    available: false,
                    version: null,
                    error: {
                      code: "not_found",
                      message: "Install ffprobe and check again.",
                    },
                  },
                ],
        },
      });
    }, mediaFixture);
    await page.route("https://media.example.org/**", (route) =>
      route.fulfill({
        contentType: "image/svg+xml",
        body: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 180"><rect width="320" height="180" fill="#23292d"/><path d="m130 55 65 35-65 35Z" fill="#bddfa1"/></svg>',
      }),
    );
    await page.goto("/");
    await expect(
      page.getByRole("heading", { name: "Add your next media" }),
    ).toBeVisible();
    await assertNoHorizontalClipping(page);
    if (viewport.width === 800) {
      const downloads = await page
        .getByRole("region", { name: "Downloads" })
        .boundingBox();
      expect(downloads!.y + downloads!.height).toBeLessThanOrEqual(
        viewport.height,
      );
    }
    await expect(
      page.getByRole("button", { name: "Preview media" }),
    ).toBeEnabled();
    await page.getByRole("textbox").fill("https://media.example.org/watch");
    await page.getByRole("textbox").press("Enter");
    await expect(
      page.getByRole("heading", { name: mediaFixture.title }),
    ).toBeVisible();
    await expect(
      page.getByRole("table", { name: "Available source formats" }),
    ).toBeVisible();
    await assertNoHorizontalClipping(page);
    if (viewport.width === 800)
      await page.screenshot({
        path: "test-results/media-preview-800x600.png",
        fullPage: true,
      });
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await expect(
      page.getByRole("table", { name: "Local tool availability" }),
    ).toBeVisible();
    await expect(
      page.getByText(/Install ffprobe and check again\./),
    ).toBeVisible();
    await assertNoHorizontalClipping(page);
    await page.getByRole("button", { name: "Downloads", exact: true }).click();
    await expect(
      page.getByRole("heading", { name: "Your queue is clear" }),
    ).toBeVisible();
    await assertNoHorizontalClipping(page);
    expect(errors).toEqual([]);
  });
}

test("keyboard reaches navigation and URL input with visible focus and a working skip link", async ({
  page,
}) => {
  await page.addInitScript((media) => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {
        invoke: async (command: string) =>
          command === "get_media_info" ? { ...media, thumbnail: null } : [],
      },
    });
  }, mediaFixture);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Add your next media" }),
  ).toBeVisible();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("link", { name: "Skip to content" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("main")).toBeFocused();
  await page.keyboard.press("Tab");
  const input = page.getByRole("textbox", { name: "Public media URL" });
  await expect(input).toBeFocused();
  expect(
    await input.evaluate((element) => getComputedStyle(element).outlineWidth),
  ).toBe("2px");
  await input.fill("https://media.example.org/watch");
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: mediaFixture.title }),
  ).toBeVisible();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("button", { name: "Preview media" }),
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("button", { name: "View downloads" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "Downloads", level: 1 }),
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("textbox")).toHaveValue(
    "https://media.example.org/watch",
  );
  // Shift+Tab walks back through the native navigation buttons.
  await page.keyboard.press("Shift+Tab");
  await expect(
    page.getByRole("button", { name: "Settings", exact: true }),
  ).toBeFocused();
});

test("idle shell stays dark and does not invoke IPC, fetch media, animate or poll", async ({
  page,
}) => {
  const externalRequests: string[] = [];
  page.on("request", (request) => {
    if (new URL(request.url()).origin !== "http://127.0.0.1:4173")
      externalRequests.push(request.url());
  });
  await page.addInitScript(() => {
    const calls: string[] = [];
    Object.defineProperty(window, "__testInvocations", { value: calls });
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {
        invoke: (command: string) => {
          calls.push(command);
          return Promise.resolve([]);
        },
      },
    });
  });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(page.getByRole("textbox")).toBeVisible();
  await page.clock.install();
  await page.clock.fastForward(60_000);
  expect(
    await page.evaluate(() => getComputedStyle(document.body).backgroundColor),
  ).toBe("rgb(16, 19, 21)");
  expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
  expect(externalRequests).toEqual([]);
  expect(
    await page.evaluate(() => Reflect.get(window, "__testInvocations")),
  ).toEqual([]);
  expect(errors).toEqual([]);
  await page.screenshot({ path: "test-results/workspace-800x600.png" });
});
