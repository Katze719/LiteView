<script lang="ts">
  import "./settings.css";
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

  type TargetItem = { index: number; id: number; title: string; kind: string };

  let error = $state<string>("");
  let capturing = $state(false);
  let settingsFps = $state(60);
  let settingsResolution = $state("captured");
  let settingsTargetIndex = $state<string>("");
  let settingsShowCursor = $state(true);
  let settingsSaved = $state(false);
  let appVersion = $state("");
  let captureTargets = $state<TargetItem[]>([]);
  let targetsLoading = $state(false);
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

  async function loadTargets() {
    if (!isTauri) return;
    targetsLoading = true;
    try {
      captureTargets = await invoke<TargetItem[]>("get_capture_targets");
    } catch {
      captureTargets = [];
    } finally {
      targetsLoading = false;
    }
  }

  async function loadSettings() {
    if (!isTauri) return;
    try {
      const s = await invoke<{
        fps: number;
        resolution: string;
        target_index: number | null;
        show_cursor: boolean;
      }>("get_capture_settings");
      settingsFps = s.fps;
      settingsResolution = s.resolution ?? "captured";
      settingsTargetIndex =
        s.target_index != null ? String(s.target_index) : "";
      settingsShowCursor = s.show_cursor ?? true;
      appVersion = await invoke<string>("get_app_version");
      await loadTargets();
    } catch {
      /* keep defaults */
    }
  }

  async function saveSettings() {
    if (!isTauri) return;
    error = "";
    try {
      await invoke("set_capture_settings", {
        fps: Number(settingsFps),
        resolution: settingsResolution,
        targetIndex:
          settingsTargetIndex === ""
            ? null
            : Number(settingsTargetIndex),
        showCursor: settingsShowCursor,
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
  <title>LiteView — Settings</title>
</svelte:head>

<div class="app">
  <header class="header" data-tauri-drag-region>
    <div class="header-brand">
      <span class="logo" aria-hidden="true">◉</span>
      <h1 class="title">LiteView</h1>
      {#if capturing}
        <span class="status-pill status-live" title="Capture is running">
          <span class="status-dot"></span>
          Live
        </span>
      {/if}
    </div>
  </header>

  <main class="main">
    {#if error}
      <div class="alert alert-error" role="alert">
        <span class="alert-icon">!</span>
        <span>{error}</span>
      </div>
    {/if}

    <section class="card">
      <h2 class="card-title">Capture</h2>
      <p class="card-desc">FPS and output resolution. Saved automatically and restored on next launch; applied on the next capture start.</p>

      <div class="field field-full">
        <label for="target">Capture target</label>
        <select
          id="target"
          bind:value={settingsTargetIndex}
          class="input"
          disabled={targetsLoading}
          onchange={() => saveSettings()}
        >
          <option value="">Default (primary)</option>
          {#each captureTargets as t}
            <option value={t.index}>
              [{#if t.kind === "display"}Display{:else}Window{/if}] {t.title || "Unnamed"}
            </option>
          {/each}
        </select>
      </div>

      <div class="form-row">
        <div class="field">
          <label for="fps">Frame rate</label>
          <select
            id="fps"
            bind:value={settingsFps}
            class="input"
            onchange={() => saveSettings()}
          >
            {#each FPS_OPTIONS as fps}
              <option value={fps}>{fps} fps</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="resolution">Resolution</label>
          <select
            id="resolution"
            bind:value={settingsResolution}
            class="input"
            onchange={() => saveSettings()}
          >
            {#each RESOLUTION_OPTIONS as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="field field-checkbox">
        <label class="checkbox-label">
          <input
            type="checkbox"
            bind:checked={settingsShowCursor}
            onchange={() => saveSettings()}
          />
          <span>Show cursor in capture</span>
        </label>
      </div>

      <button
        type="button"
        class="btn btn-primary"
        onclick={saveSettings}
        aria-pressed={settingsSaved}
      >
        {#if settingsSaved}
          <span class="btn-icon">✓</span>
          Saved
        {:else}
          Save settings
        {/if}
      </button>
    </section>

    <section class="card card-muted">
      <h2 class="card-title">How to use</h2>
      <p class="card-desc">
        Choose a <strong>capture target</strong> above (display or window), then click the <strong>tray icon</strong> and <strong>Start capture…</strong>.
        The preview opens in a separate window. Use <strong>Stop capture</strong> when done.
      </p>
      <p class="card-desc">
        Closing this window hides it to the tray; the app keeps running until you choose <strong>Quit LiteView</strong>.
      </p>
    </section>

    {#if appVersion}
      <section class="card card-compact about">
        <h2 class="card-title">About</h2>
        <p class="about-version">LiteView v{appVersion}</p>
        <p class="about-desc">Lightweight screen preview via system tray.</p>
      </section>
    {/if}
  </main>
</div>
