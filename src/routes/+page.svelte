<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";

  interface ScapTarget {
    index: number;
    id: number;
    title: string;
    kind: string;
  }

  let error = $state<string>("");
  let capturing = $state(false);
  let scapTargets = $state<ScapTarget[]>([]);
  let scapTargetsLoaded = $state(false);
  let scapError = $state<string>("");
  let selectedScapIndex = $state<number>(0);
  let unlistenScapError: (() => void) | null = null;

  const isTauri =
    typeof (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !== "undefined";

  /** Extract error message from Tauri invoke rejection (string, Error, or { message }). */
  function getInvokeError(e: unknown): string {
    if (typeof e === "string") return e;
    if (e instanceof Error) return e.message;
    if (e != null && typeof e === "object" && "message" in e && typeof (e as { message: unknown }).message === "string")
      return (e as { message: string }).message;
    return String(e);
  }

  function stopCapture() {
    if (isTauri) {
      invoke("stop_scap_capture").catch(() => {});
    }
    unlistenScapError?.();
    unlistenScapError = null;
    capturing = false;
    error = "";
  }

  /** Start capture. Preview opens in native window (tao + wgpu). */
  async function startCaptureWithScap() {
    error = "";
    stopCapture();
    const targetIndex =
      scapTargets.length > 0 ? Number(selectedScapIndex) : null;
    try {
      await invoke("start_scap_capture", { targetIndex });
      capturing = true;
      unlistenScapError = await listen("scap-error", (event) => {
        error = String(event.payload);
      });
    } catch (e) {
      error = getInvokeError(e);
    }
  }

  /** Called e.g. from tray "capture-start"; only starts scap capture. */
  async function startCapture() {
    await startCaptureWithScap();
  }

  const LOAD_TARGETS_TIMEOUT_MS = 15_000;

  function loadScapTargets() {
    scapTargetsLoaded = false;
    scapError = "";
    const loadPromise = invoke<ScapTarget[]>("get_scap_targets")
      .then((targets) => {
        scapTargets = targets ?? [];
        scapError = "";
        scapTargetsLoaded = true;
        if (scapTargets.length > 0) selectedScapIndex = 0;
      })
      .catch((e) => {
        scapError = getInvokeError(e);
        scapTargetsLoaded = true;
      });
    const timeoutPromise = new Promise<void>((resolve) =>
      setTimeout(() => resolve(), LOAD_TARGETS_TIMEOUT_MS)
    );
    Promise.race([loadPromise, timeoutPromise]).then(() => {
      if (!scapTargetsLoaded) {
        scapTargetsLoaded = true;
        scapError =
          "Loading timed out. Check that the system screen-sharing dialog appeared, or try again.";
      }
    });
  }

  onMount(() => {
    if (isTauri) {
      loadScapTargets();
      listen("capture-start", () => startCapture()).then((fn) => {
        unlistenStart = fn;
      });
      listen("capture-stop", () => stopCapture()).then((fn) => {
        unlistenStop = fn;
      });
    }
    let unlistenStart: (() => void) | null = null;
    let unlistenStop: (() => void) | null = null;
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
    {:else}
      {#if scapTargets.length > 0}
        <select
          class="display-select"
          bind:value={selectedScapIndex}
          aria-label="Select screen or window"
        >
          {#each scapTargets as t}
            <option value={t.index}>{t.title} ({t.kind})</option>
          {/each}
        </select>
        <button type="button" class="btn btn-start" onclick={startCaptureWithScap}>
          Start capture
        </button>
      {:else if isTauri && scapTargetsLoaded && !scapError}
        <button type="button" class="btn btn-start" onclick={startCaptureWithScap}>
          Select screen / Start capture
        </button>
      {/if}
    {/if}
  </header>

  <main class="preview">
    {#if capturing}
      <p class="hint">Preview runs in the separate “LiteView Preview” window (native, low latency).</p>
    {/if}
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
    {#if isTauri && scapTargetsLoaded && scapTargets.length === 0 && scapError && !capturing}
      <p class="error" role="alert">{scapError}</p>
      <p class="hint">
        Don't run with sudo. On Linux Wayland: ensure PipeWire and
        xdg-desktop-portal are installed; grant screen-sharing when the system
        prompts you.
      </p>
      <button type="button" class="btn btn-start retry-btn" onclick={loadScapTargets}>
        Retry
      </button>
    {:else if !capturing && !error}
      <p class="hint">
        {#if scapTargets.length > 0}
          Select a screen/window above and click “Start capture”.
        {:else if isTauri && !scapTargetsLoaded}
          Loading capture targets…
        {:else if isTauri && scapTargetsLoaded && scapTargets.length === 0}
          On Linux, click “Select screen / Start capture” above. A system dialog will let you choose which screen or window to capture.
        {:else}
          Run as desktop app (pnpm tauri dev) for screen capture.
        {/if}
      </p>
      {#if isTauri && scapTargetsLoaded && scapTargets.length === 0 && scapError}
        <button type="button" class="btn btn-start retry-btn" onclick={loadScapTargets}>
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
