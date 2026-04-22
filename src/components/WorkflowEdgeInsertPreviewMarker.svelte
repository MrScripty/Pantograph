<script lang="ts">
  interface Props {
    hitPoint: {
      x: number;
      y: number;
    };
  }

  let { hitPoint }: Props = $props();
</script>

<div
  class="edge-insert-preview-marker"
  style="
    left: {hitPoint.x}px;
    top: {hitPoint.y}px;
  "
  aria-hidden="true"
>
  <div class="edge-insert-preview-marker-core"></div>
</div>

<style>
  .edge-insert-preview-marker {
    position: absolute;
    width: 22px;
    height: 22px;
    margin-left: -11px;
    margin-top: -11px;
    border: 1px solid rgba(186, 230, 253, 0.9);
    border-radius: 999px;
    background:
      radial-gradient(circle, rgba(125, 211, 252, 0.24) 0%, rgba(125, 211, 252, 0.08) 58%, transparent 72%);
    box-shadow:
      0 0 0 1px rgba(14, 116, 144, 0.35),
      0 0 16px rgba(125, 211, 252, 0.28);
    pointer-events: none;
    z-index: 1200;
  }

  .edge-insert-preview-marker-core {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 8px;
    height: 8px;
    margin-top: -4px;
    margin-left: -4px;
    border-radius: 999px;
    background: #e0f2fe;
    box-shadow: 0 0 10px rgba(224, 242, 254, 0.8);
  }
</style>
