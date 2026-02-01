<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";

  interface CaptureTarget {
    index: number;
    id: number;
    name: string;
    kind: string;
  }

  const LOAD_TARGETS_TIMEOUT_MS = 15_000;
  const isTauri =
    typeof (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !==
    "undefined";

  let error = $state<string>("");
  let capturing = $state(false);
  let captureTargets = $state<CaptureTarget[]>([]);
  let targetsLoaded = $state(false);
  let targetsError = $state<string>("");
  let selectedTargetIndex = $state<number>(0);
  let unlistenError: (() => void) | null = null;

  function getInvokeError(e: unknown): string {
    if (typeof e === "string") return e;
    if (e instanceof Error) return e.message;
    if (
      e != null &&
      typeof e === "object" &&
      "message" in e &&
      typeof (e as { message: unknown }).message === "string"
    )
      return (e as { message: string }).message;
    return String(e);
  }

  function stopCapture() {
    if (isTauri) invoke("stop_capture").catch(() => {});
    unlistenError?.();
    unlistenError = null;
    capturing = false;
    error = "";
  }

  async function startCapture() {
    error = "";
    stopCapture();
    const targetIndex = captureTargets.length > 0 ? Number(selectedTargetIndex) : null;
    try {
      await invoke("start_capture", { targetIndex });
      capturing = true;
      unlistenError = await listen("capture-error", (event) => {
        error = String(event.payload);
      });
    } catch (e) {
      error = getInvokeError(e);
    }
  }

  function loadCaptureTargets() {
    targetsLoaded = false;
    targetsError = "";
    const loadPromise = invoke<CaptureTarget[]>("get_capture_targets")
      .then((targets) => {
        captureTargets = targets ?? [];
        targetsError = "";
        targetsLoaded = true;
        if (targets?.length) selectedTargetIndex = 0;
      })
      .catch((e) => {
        targetsError = getInvokeError(e);
        targetsLoaded = true;
      });
    const timeoutPromise = new Promise<void>((r) =>
      setTimeout(r, LOAD_TARGETS_TIMEOUT_MS)
    );
    Promise.race([loadPromise, timeoutPromise]).then(() => {
      if (!targetsLoaded) {
        targetsLoaded = true;
        targetsError =
          "Loading timed out. Check that the system screen-sharing dialog appeared, or try again.";
      }
    });
  }

  onMount(() => {
    let unlistenStart: (() => void) | null = null;
    let unlistenStop: (() => void) | null = null;

    if (isTauri) {
      loadCaptureTargets();
      listen("capture-start", startCapture).then((fn) => (unlistenStart = fn));
      listen("capture-stop", stopCapture).then((fn) => (unlistenStop = fn));
    }

    return () => {
      unlistenStart?.();
      unlistenStop?.();
      stopCapture();
    };
  });
</script>

<svelte:head>
  <title>LiteView</title>
</svelte:head>

<div class="pip">
  <header class="header" data-tauri-drag-region>
    <span class="title">LiteView</span>
    {#if capturing}
      <button type="button" class="btn btn-stop" onclick={stopCapture}>Stop</button>
    {:else if captureTargets.length > 0}
      <select
        class="display-select"
        bind:value={selectedTargetIndex}
        aria-label="Select screen or window"
      >
        {#each captureTargets as t}
          <option value={t.index}>{t.name} ({t.kind})</option>
        {/each}
      </select>
      <button type="button" class="btn btn-start" onclick={startCapture}>
        Start capture
      </button>
    {:else if isTauri && targetsLoaded && !targetsError}
      <button type="button" class="btn btn-start" onclick={startCapture}>
        Select screen / Start capture
      </button>
    {/if}
  </header>

  <main class="preview">
    {#if capturing}
      <p class="hint">
        Preview runs in the separate "LiteView Preview" window (native, low latency).
      </p>
    {/if}
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
    {#if isTauri && targetsLoaded && captureTargets.length === 0 && targetsError && !capturing}
      <p class="error" role="alert">{targetsError}</p>
      <p class="hint">
        Ensure screen capture permissions are granted.
      </p>
      <button type="button" class="btn btn-start retry-btn" onclick={loadCaptureTargets}>
        Retry
      </button>
    {:else if !capturing && !error}
      <p class="hint">
        {#if captureTargets.length > 0}
          Select a screen above and click "Start capture".
        {:else if isTauri && !targetsLoaded}
          Loading capture targets…
        {:else if isTauri && targetsLoaded && captureTargets.length === 0}
          Click "Select screen / Start capture" above.
        {:else}
          Run as desktop app (pnpm tauri dev) for screen capture.
        {/if}
      </p>
      {#if isTauri && targetsLoaded && captureTargets.length === 0 && targetsError}
        <button type="button" class="btn btn-start retry-btn" onclick={loadCaptureTargets}>
          Retry
        </button>
      {/if}
    {/if}
  </main>
</div>

<style>
  .pip {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--fg);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    background: var(--header-bg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .title {
    font-size: 0.875rem;
    font-weight: 600;
  }

  .display-select {
    flex: 1;
    min-width: 0;
    padding: 0.35rem 0.5rem;
    font-size: 0.8rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--input-bg);
    color: var(--fg);
    cursor: pointer;
  }

  .btn {
    padding: 0.35rem 0.75rem;
    font-size: 0.8rem;
    border-radius: 6px;
    border: 1px solid var(--border);
    cursor: pointer;
    font-family: inherit;
  }

  .btn-start {
    background: var(--accent);
    color: var(--accent-fg);
    border-color: var(--accent);
  }

  .btn-start:hover {
    filter: brightness(1.1);
  }

  .btn-stop {
    background: var(--input-bg);
    color: var(--fg);
  }

  .btn-stop:hover {
    background: var(--hover);
  }

  .retry-btn {
    margin-top: 0.75rem;
  }

  .preview {
    flex: 1;
    min-height: 0;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--preview-bg);
    overflow: hidden;
  }

  .error {
    position: absolute;
    bottom: 1rem;
    left: 1rem;
    right: 1rem;
    color: var(--error);
    font-size: 0.875rem;
    text-align: center;
    margin: 0;
  }

  .hint {
    position: absolute;
    color: var(--muted);
    font-size: 0.875rem;
    margin: 0;
  }

  :global(:root) {
    --bg: #1a1a1a;
    --fg: #e5e5e5;
    --header-bg: #252525;
    --border: #3a3a3a;
    --input-bg: #2a2a2a;
    --preview-bg: #0d0d0d;
    --muted: #737373;
    --error: #f87171;
    --accent: #3b82f6;
    --accent-fg: #fff;
    --hover: #333;
  }

  @media (prefers-color-scheme: light) {
    :global(:root) {
      --bg: #f5f5f5;
      --fg: #171717;
      --header-bg: #e5e5e5;
      --border: #d4d4d4;
      --input-bg: #fff;
      --preview-bg: #262626;
      --muted: #737373;
      --error: #dc2626;
      --accent: #2563eb;
      --accent-fg: #fff;
      --hover: #e5e5e5;
    }
  }
</style>
