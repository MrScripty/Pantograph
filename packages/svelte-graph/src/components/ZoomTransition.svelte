<script lang="ts">
  import { scale, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { useGraphContext } from '../context/useGraphContext.js';
  import type { ViewLevel } from '../types/view.js';

  const { stores } = useGraphContext();
  const { viewLevel, animationConfig, zoomTarget, isAnimating } = stores.view;

  interface Props {
    level: ViewLevel;
    children?: import('svelte').Snippet;
  }

  let { level, children }: Props = $props();

  let isVisible = $derived($viewLevel === level);

  let transitionParams = $derived.by(() => {
    const config = $animationConfig;
    const target = $zoomTarget;

    let startScale = 0.1;
    if (level === 'orchestration') {
      startScale = 2;
    } else if (level === 'data-graph') {
      startScale = 0.1;
    } else if (level === 'group') {
      startScale = 0.1;
    }

    return {
      duration: config.duration,
      easing: cubicOut,
      start: startScale,
      opacity: 0,
    };
  });

  let transformOrigin = $derived.by(() => {
    const target = $zoomTarget;
    if (target && target.position) {
      return `${target.position.x}px ${target.position.y}px`;
    }
    return 'center center';
  });
</script>

<div
  class="zoom-transition-container"
  class:animating={$isAnimating}
  style:--transform-origin={transformOrigin}
  style:--animation-duration="{$animationConfig.duration}ms"
>
  {#if isVisible}
    <div
      class="zoom-content"
      in:scale={transitionParams}
      out:scale={{ ...transitionParams, start: transitionParams.start === 0.1 ? 2 : 0.1 }}
    >
      {#if children}
        {@render children()}
      {/if}
    </div>
  {/if}
</div>

<style>
  .zoom-transition-container {
    position: absolute;
    inset: 0;
    overflow: hidden;
    transform-origin: var(--transform-origin, center center);
  }

  .zoom-content {
    width: 100%;
    height: 100%;
    transform-origin: var(--transform-origin, center center);
  }

  .animating {
    pointer-events: none;
  }

  .zoom-content {
    will-change: transform, opacity;
    backface-visibility: hidden;
  }

  .animating .zoom-content {
    filter: blur(0px);
    transition: filter var(--animation-duration) ease-out;
  }
</style>
