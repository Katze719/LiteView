<script lang="ts">
  import { onMount } from "svelte";

  let videoEl = $state<HTMLVideoElement | null>(null);
  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let stream = $state<MediaStream | null>(null);
  let error = $state<string>("");
  let capturing = $state(false);
  let rafId = $state<number | null>(null);

  function drawFrame() {
    if (!videoEl || !canvasEl || !stream || videoEl.readyState < 2) return;
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;
    const vw = videoEl.videoWidth;
    const vh = videoEl.videoHeight;
    if (vw === 0 || vh === 0) return;
    if (canvasEl.width !== vw || canvasEl.height !== vh) {
      canvasEl.width = vw;
      canvasEl.height = vh;
    }
    ctx.drawImage(videoEl, 0, 0);
    rafId = requestAnimationFrame(drawFrame);
  }

  function startDrawing() {
    if (!videoEl || !canvasEl || !stream) return;
    rafId = requestAnimationFrame(drawFrame);
  }

  function stopDrawing() {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  }

  function stopCapture() {
    stopDrawing();
    if (stream) {
      stream.getTracks().forEach((t) => t.stop());
      stream = null;
    }
    if (videoEl) {
      videoEl.srcObject = null;
    }
    capturing = false;
    error = "";
  }

  async function startCapture() {
    error = "";
    // Call getDisplayMedia immediately (no await before) so the user gesture
    // is still active in WebKit – otherwise some Linux/Wayland setups throw
    // NotAllowedError after the portal returns.
    const streamPromise = navigator.mediaDevices.getDisplayMedia({
      video: true,
      audio: false,
    });
    try {
      const mediaStream = await streamPromise;
      stopCapture();
      stream = mediaStream;
      capturing = true;
      const video = videoEl;
      if (video) {
        video.srcObject = mediaStream;
        video.onloadedmetadata = () => {
          video.play().catch(() => {});
          startDrawing();
        };
      }
      mediaStream.getVideoTracks()[0]?.addEventListener("ended", stopCapture);
    } catch (e) {
      const err = e as DOMException;
      if (err?.name === "NotAllowedError") {
        error =
          "Screen sharing was denied or failed. On Linux Wayland this can happen " +
          "after selecting a screen (WebKit/portal quirk). See README for workarounds.";
      } else {
        error = err?.message ?? String(e);
      }
    }
  }

  onMount(() => {
    return () => {
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
      <button type="button" class="btn btn-start" onclick={startCapture}>
        Select screen/window
      </button>
    {/if}
  </header>

  <main class="preview">
    <video
      bind:this={videoEl}
      class="video-src"
      muted
      playsinline
      autoplay
      aria-hidden="true"
    ></video>
    <canvas
      bind:this={canvasEl}
      class="preview-canvas"
      aria-label="Screen preview"
    ></canvas>
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
    {#if !capturing && !error}
      <p class="hint">Click “Select screen/window” to show another screen in this window.</p>
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

  .video-src {
    position: absolute;
    width: 0;
    height: 0;
    opacity: 0;
    pointer-events: none;
  }

  .preview-canvas {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    display: block;
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
