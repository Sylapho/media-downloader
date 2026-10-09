<script lang="ts">
  import { tick } from "svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Button from "$lib/components/ui/button/Button.svelte";
  import Input from "$lib/components/ui/input/Input.svelte";
  import DependencyStatus from "$lib/DependencyStatus.svelte";
  import MediaPreview from "$lib/components/MediaPreview.svelte";
  import { createMediaPreview } from "$lib/media-preview.svelte";

  let view = $state<"new" | "downloads" | "settings">("new");
  const preview = createMediaPreview();
  let heading: HTMLHeadingElement;

  const navigation = [
    { id: "new", label: "New media", icon: "plus" },
    { id: "downloads", label: "Downloads", icon: "download" },
    { id: "settings", label: "Settings", icon: "settings" },
  ] as const;

  async function navigate(next: typeof view) {
    view = next;
    await tick();
    heading?.focus();
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    void preview.analyze();
  }
</script>

<svelte:head>
  <title>Media Downloader</title>
  <meta
    name="description"
    content="Your desktop workspace for media downloads."
  />
</svelte:head>

<a
  href="#workspace"
  class="fixed top-3 left-3 z-50 -translate-y-24 rounded-lg bg-primary px-4 py-3 text-primary-foreground focus:translate-y-0"
  >Skip to content</a
>

<div class="min-h-dvh md:grid md:grid-cols-[176px_minmax(0,1fr)]">
  <aside class="flex flex-col border-b bg-card/60 md:border-r md:border-b-0">
    <div class="flex items-center gap-3 px-5 py-5 md:py-7">
      <span
        class="flex size-9 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground"
      >
        <Icon name="download" />
      </span>
      <span class="text-sm leading-tight font-semibold tracking-tight"
        >Media<br class="hidden md:block" /> Downloader</span
      >
    </div>
    <nav aria-label="Main navigation" class="flex gap-1 px-3 pb-3 md:flex-col">
      {#each navigation as item (item.id)}
        <Button
          variant="ghost"
          class={view === item.id
            ? "min-w-0 flex-1 justify-start bg-secondary text-foreground md:flex-none"
            : "min-w-0 flex-1 justify-start md:flex-none"}
          aria-current={view === item.id ? "page" : undefined}
          onclick={() => navigate(item.id)}
        >
          <Icon
            name={item.icon}
            class="hidden size-4 shrink-0 min-[400px]:block"
          />
          {item.label}
        </Button>
      {/each}
    </nav>
    <div
      class="mt-auto hidden px-5 py-5 text-xs text-muted-foreground md:block"
    >
      <p>Desktop workspace</p>
      <p class="mt-1 font-mono">v0.1.0 · Early access</p>
    </div>
  </aside>

  <main
    id="workspace"
    tabindex="-1"
    class="min-w-0 px-5 py-5 sm:px-7 lg:px-10 lg:py-9"
  >
    <div class="mx-auto max-w-4xl">
      <header class="mb-5 flex flex-wrap items-start justify-between gap-3">
        <div>
          <p
            class="mb-1 text-xs font-medium tracking-[0.16em] text-muted-foreground uppercase"
          >
            Your workspace
          </p>
          <h1
            bind:this={heading}
            tabindex="-1"
            class="w-fit rounded-sm text-2xl font-semibold tracking-tight"
          >
            {view === "new"
              ? "Add your next media"
              : view === "downloads"
                ? "Downloads"
                : "Settings"}
          </h1>
          <p class="mt-1 text-sm text-muted-foreground">
            {view === "new"
              ? "A link is the starting point. Keep everything in one place."
              : view === "downloads"
                ? "Your downloads will appear here."
                : "Check the tools available to your desktop app."}
          </p>
        </div>
        <span
          class="rounded-full border px-3 py-1 text-xs text-muted-foreground"
          >Early access</span
        >
      </header>

      {#if view === "new"}
        <section
          aria-labelledby="url-heading"
          class="rounded-xl border bg-card p-4"
        >
          <h2 id="url-heading" class="mb-3 font-medium">Media URL</h2>
          <form
            novalidate
            onsubmit={submit}
            class="flex flex-col gap-3 min-[600px]:flex-row"
          >
            <div class="relative min-w-0 flex-1">
              <span
                class="pointer-events-none absolute top-3.5 left-3 text-muted-foreground"
                ><Icon name="link" class="size-4" /></span
              >
              <label for="media-url" class="sr-only">Public media URL</label>
              <Input
                id="media-url"
                type="url"
                bind:value={preview.url}
                placeholder="https://…"
                aria-describedby="url-help"
                aria-invalid={preview.error?.code === "invalid_url"
                  ? true
                  : undefined}
                aria-errormessage={preview.error?.code === "invalid_url"
                  ? "preview-error"
                  : undefined}
                autocomplete="off"
                spellcheck="false"
                class="pl-9"
              />
            </div>
            <Button
              type="submit"
              disabled={preview.pending}
              aria-describedby="url-help"
              ><Icon name="arrow" class="size-4" />{preview.pending
                ? "Analyzing…"
                : "Preview media"}</Button
            >
          </form>
          <p
            id="url-help"
            class="mt-3 text-xs leading-relaxed text-muted-foreground"
          >
            Public media only. Analysis fetches metadata; it does not start a
            download.
          </p>
        </section>

        <MediaPreview
          media={preview.media}
          pending={preview.pending}
          error={preview.error}
          changedWhilePending={preview.changedWhilePending}
          onsettings={() => navigate("settings")}
        />

        <section
          aria-labelledby="downloads-heading"
          class="mt-5 flex flex-wrap items-center justify-between gap-3 rounded-xl border px-5 py-3"
        >
          <div class="flex min-w-0 items-center gap-3">
            <Icon
              name="download"
              class="size-5 shrink-0 text-muted-foreground"
            />
            <div>
              <h2 id="downloads-heading" class="font-medium">Downloads</h2>
              <p class="mt-1 text-xs text-muted-foreground">No downloads yet</p>
            </div>
          </div>
          <Button variant="ghost" onclick={() => navigate("downloads")}
            >View downloads <Icon name="arrow" class="size-4" /></Button
          >
        </section>
      {:else if view === "downloads"}
        <section
          aria-labelledby="queue-heading"
          class="rounded-xl border bg-card px-6 py-12 text-center"
        >
          <span
            class="mx-auto mb-5 flex size-14 items-center justify-center rounded-2xl border bg-secondary text-muted-foreground"
            ><Icon name="download" class="size-7" /></span
          >
          <h2 id="queue-heading" class="text-lg font-medium">
            Your queue is clear
          </h2>
          <p
            class="mx-auto mt-3 max-w-sm leading-relaxed text-muted-foreground"
          >
            Download controls and progress will live here. Downloads are not
            available in this version.
          </p>
          <Button
            variant="secondary"
            class="mt-6"
            onclick={() => navigate("new")}
            ><Icon name="plus" class="size-4" />Back to new media</Button
          >
        </section>
      {:else}
        <section
          aria-labelledby="appearance-heading"
          class="mb-5 flex flex-wrap items-center justify-between gap-3 rounded-xl border bg-card px-5 py-4"
        >
          <div>
            <h2 id="appearance-heading" class="font-medium">Appearance</h2>
            <p class="mt-1 text-xs text-muted-foreground">
              A calm workspace, day or night.
            </p>
          </div>
          <span class="rounded-lg border bg-background px-3 py-2 text-sm"
            >Dark theme</span
          >
        </section>
        <DependencyStatus />
      {/if}
    </div>
  </main>
</div>
