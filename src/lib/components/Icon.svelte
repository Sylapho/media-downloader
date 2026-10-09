<script lang="ts">
  let {
    name,
    class: className = "size-5",
  }: {
    name:
      | "download"
      | "plus"
      | "settings"
      | "link"
      | "play"
      | "arrow"
      | "check"
      | "alert";
    class?: string;
  } = $props();

  const paths = {
    download: [
      "M12 3v12m-5-5 5 5 5-5",
      "M4 15v5a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-5",
    ],
    plus: ["M12 5v14M5 12h14"],
    settings: [
      "M9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1 1-3Z",
      "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z",
    ],
    link: [
      "m9 15 6-6",
      "M7 14 5 16a3 3 0 0 0 4 4l4-4a3 3 0 0 0 0-4",
      "m17 10 2-2a3 3 0 0 0-4-4l-4 4a3 3 0 0 0 0 4",
    ],
    play: ["m9 6 10 6-10 6V6Z"],
    arrow: ["M4 12h16m-6-6 6 6-6 6"],
    check: ["m5 12 4 4L19 6"],
    alert: ["m12 3 10 18H2L12 3Z", "M12 9v4m0 3v1"],
  };
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.7"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
  class={className}
>
  {#each paths[name] as path (path)}
    <path d={path} />
  {/each}
</svg>
