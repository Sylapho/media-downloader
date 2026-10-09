<script lang="ts">
  import { onMount } from "svelte";
  import { createDependencyCheck } from "./dependency-check.svelte";
  import Button from "$lib/components/ui/button/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";

  const model = createDependencyCheck();
  let statuses = $derived(model.statuses);
  let checking = $derived(model.checking);
  let error = $derived(model.error);

  onMount(() => {
    void model.check();
  });
</script>

<section
  aria-labelledby="dependencies-heading"
  aria-busy={checking}
  class="min-w-0 rounded-xl border bg-card p-5"
>
  <div class="flex flex-wrap items-center justify-between gap-3">
    <h2 id="dependencies-heading" class="font-medium">Media dependencies</h2>
    <Button variant="secondary" onclick={model.check} disabled={checking}>
      {checking ? "Checking dependencies…" : "Check again"}
    </Button>
  </div>
  <p class="mt-3 text-xs leading-relaxed text-muted-foreground">
    Required: yt-dlp ≥ 2024.01.01, FFmpeg and ffprobe ≥ 6.0 (numbered releases).
  </p>
  <div aria-live="polite" class="mt-4">
    {#if checking}
      <p
        role="status"
        class="rounded-lg border bg-background p-4 text-muted-foreground"
      >
        Checking local tools…
      </p>
    {:else if error}
      <p
        role="alert"
        class="flex items-start gap-2 rounded-lg border border-destructive/40 bg-destructive/5 p-4 text-destructive"
      >
        <Icon name="alert" class="mt-0.5 size-4 shrink-0" />{error}
      </p>
    {:else if statuses.length > 0}
      <table class="w-full table-fixed text-left text-sm">
        <caption class="sr-only">Local tool availability</caption>
        <thead
          ><tr
            ><th
              scope="col"
              class="w-1/4 py-3 font-medium text-muted-foreground">Tool</th
            ><th
              scope="col"
              class="w-1/4 py-3 font-medium text-muted-foreground">Status</th
            ><th scope="col" class="py-3 font-medium text-muted-foreground"
              >Version</th
            ></tr
          ></thead
        >
        <tbody>
          {#each statuses as status (status.tool)}
            <tr
              ><th
                scope="row"
                class="border-t py-3 pr-2 font-medium [overflow-wrap:anywhere]"
                >{status.tool}</th
              ><td class="border-t py-3 pr-2 [overflow-wrap:anywhere]"
                ><span
                  class={status.available ? "text-primary" : "text-destructive"}
                  >{status.available ? "Available" : "Unavailable"}</span
                ></td
              ><td
                class="border-t py-3 font-mono text-xs text-muted-foreground [overflow-wrap:anywhere]"
                >{status.version ?? "Not detected"}</td
              ></tr
            >
          {/each}
        </tbody>
      </table>
      {#each statuses.filter((status) => !status.available) as status (status.tool)}
        <p
          class="mt-3 rounded-lg border border-destructive/30 p-3 text-sm leading-relaxed [overflow-wrap:anywhere]"
        >
          <strong>{status.tool}:</strong>
          {status.error?.message ?? "Install the tool and check again."}
        </p>
      {/each}
      {#if statuses.some((status) => !status.available)}
        <p class="mt-4 text-xs leading-relaxed text-muted-foreground">
          Install <a
            href="https://github.com/yt-dlp/yt-dlp#installation"
            target="_blank"
            class="rounded-sm text-primary underline underline-offset-4"
            rel="noreferrer">yt-dlp</a
          >
          and a
          <a
            href="https://ffmpeg.org/download.html"
            target="_blank"
            class="rounded-sm text-primary underline underline-offset-4"
            rel="noreferrer">FFmpeg package</a
          > containing both ffmpeg and ffprobe. On Windows, add their executable folders
          to PATH and restart the app, or configure absolute paths as described in
          the README. Nothing is downloaded automatically.
        </p>
      {/if}
    {:else}
      <p
        role="status"
        class="rounded-lg border bg-background p-4 text-muted-foreground"
      >
        No tool information returned. Select Check again to retry.
      </p>
    {/if}
  </div>
</section>
