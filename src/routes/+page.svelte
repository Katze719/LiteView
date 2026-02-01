<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const isTauri =
    typeof (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !==
    "undefined";

  const FPS_OPTIONS = [15, 30, 60, 90, 120] as const;
  const RESOLUTION_OPTIONS = [
    { value: "captured", label: "Native (captured)" },
    { value: "480p", label: "480p" },
    { value: "720p", label: "720p" },
    { value: "1080p", label: "1080p" },
    { value: "1440p", label: "1440p" },
    { value: "2160p", label: "2160p (4K)" },
    { value: "4320p", label: "4320p (8K)" },
  ] as const;

  let error = $state<string>("");
  let capturing = $state(false);
  let settingsFps = $state(60);
  let settingsResolution = $state("captured");
  let settingsSaved = $state(false);
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

  async function startCaptureFromTray() {
    error = "";
    stopCapture();
    try {
      await invoke("start_capture", { targetIndex: null });
      capturing = true;
      unlistenError = await listen("capture-error", (event) => {
        error = String(event.payload);
      });
    } catch (e) {
      error = getInvokeError(e);
    }
  }

  async function loadSettings() {
    if (!isTauri) return;
    try {
      const s = await invoke<{ fps: number; resolution: string }>("get_capture_settings");
      settingsFps = s.fps;
      settingsResolution = s.resolution ?? "captured";
    } catch {
      // keep defaults
    }
  }

  async function saveSettings() {
    if (!isTauri) return;
    error = "";
    try {
      await invoke("set_capture_settings", {
        fps: Number(settingsFps),
        resolution: settingsResolution,
      });
      settingsSaved = true;
      setTimeout(() => (settingsSaved = false), 1500);
    } catch (e) {
      error = getInvokeError(e);
    }
  }

  onMount(() => {
    let unlistenStart: (() => void) | null = null;
    let unlistenStop: (() => void) | null = null;
    let unlistenClose: (() => void) | null = null;

    if (isTauri) {
      loadSettings();
      listen("capture-start", startCaptureFromTray).then((fn) => (unlistenStart = fn));
      listen("capture-stop", stopCapture).then((fn) => (unlistenStop = fn));

      getCurrentWindow()
        .onCloseRequested(async (event) => {
          event.preventDefault();
          await getCurrentWindow().hide();
        })
        .then((fn) => (unlistenClose = fn));
    }

    return () => {
      unlistenStart?.();
      unlistenStop?.();
      unlistenClose?.();
      stopCapture();
    };
  });
</script>

<svelte:head>
  <title>LiteView Settings</title>
</svelte:head>

<div class="settings">
  <header class="header" data-tauri-drag-region>
    <span class="title">LiteView Settings</span>
  </header>

  <main class="main">
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <section class="section" aria-labelledby="capture-settings-heading">
      <h2 id="capture-settings-heading" class="section-title">Capture settings</h2>
      <p class="hint">Applied the next time you start capture from the tray.</p>

      <div class="field">
        <label for="fps">FPS</label>
        <select id="fps" bind:value={settingsFps} class="select">
          {#each FPS_OPTIONS as fps}
            <option value={fps}>{fps}</option>
          {/each}
        </select>
      </div>

      <div class="field">
        <label for="resolution">Resolution</label>
        <select id="resolution" bind:value={settingsResolution} class="select">
          {#each RESOLUTION_OPTIONS as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>

      <button type="button" class="btn btn-save" onclick={saveSettings}>
        {settingsSaved ? "Saved" : "Save settings"}
      </button>
    </section>

    <section class="section">
      <p class="hint">
        Use the system tray to <strong>Start</strong> (PipeWire/screen selection) or <strong>Stop</strong> capture.
        The preview appears in a separate window when capture is running.
      </p>
      {#if capturing}
        <p class="status">Capture is running.</p>
      {/if}
    </section>
  </main>
</div>

<style>
  .settings {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--fg);
  }

  .header {
    display: flex;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background: var(--header-bg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .title {
    font-size: 0.875rem;
    font-weight: 600;
  }

  .main {
    flex: 1;
    padding: 1rem 0.75rem;
    overflow: auto;
  }

  .section {
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    margin: 0 0 0.25rem 0;
  }

  .field {
    margin-top: 0.75rem;
  }

  .field label {
    display: block;
    font-size: 0.8rem;
    color: var(--muted);
    margin-bottom: 0.25rem;
  }

  .select {
    width: 100%;
    max-width: 12rem;
    padding: 0.35rem 0.5rem;
    font-size: 0.8rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--input-bg);
    color: var(--fg);
    cursor: pointer;
  }

  .btn-save {
    margin-top: 0.75rem;
    padding: 0.35rem 0.75rem;
    font-size: 0.8rem;
    border-radius: 6px;
    border: 1px solid var(--border);
    cursor: pointer;
    font-family: inherit;
    background: var(--accent);
    color: var(--accent-fg);
    border-color: var(--accent);
  }

  .btn-save:hover {
    filter: brightness(1.1);
  }

  .error {
    color: var(--error);
    font-size: 0.875rem;
    margin: 0 0 0.75rem 0;
  }

  .hint {
    color: var(--muted);
    font-size: 0.875rem;
    margin: 0;
    line-height: 1.5;
  }

  .status {
    margin-top: 0.75rem;
    font-size: 0.875rem;
    color: var(--accent);
  }

  :global(:root) {
    --bg: #1a1a1a;
    --fg: #e5e5e5;
    --header-bg: #252525;
    --border: #3a3a3a;
    --preview-bg: #0d0d0d;
    --muted: #737373;
    --error: #f87171;
    --accent: #3b82f6;
    --input-bg: #2a2a2a;
  }

  @media (prefers-color-scheme: light) {
    :global(:root) {
      --bg: #f5f5f5;
      --fg: #171717;
      --header-bg: #e5e5e5;
      --border: #d4d4d4;
      --muted: #737373;
      --error: #dc2626;
      --accent: #2563eb;
      --input-bg: #fff;
    }
  }
</style>
