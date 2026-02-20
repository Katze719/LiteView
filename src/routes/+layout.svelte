<script lang="ts">
  import "../app.css";
</script>

<div class="app-shell">
  <div class="bg-shapes" aria-hidden="true">
    <span class="shape shape-circle shape-pastel-pink"></span>
    <span class="shape shape-circle shape-pastel-mint"></span>
    <span class="shape shape-circle shape-pastel-lavender"></span>
    <span class="shape shape-square shape-pastel-peach"></span>
    <span class="shape shape-square shape-pastel-blue"></span>
    <span class="shape shape-triangle shape-pastel-yellow"></span>
    <span class="shape shape-triangle shape-pastel-pink"></span>
    <span class="shape shape-diamond shape-pastel-mint"></span>
    <span class="shape shape-blob shape-pastel-lavender"></span>
    <span class="shape shape-hexagon shape-pastel-peach"></span>
  </div>
  <main class="app-content">
    <slot />
  </main>
</div>
