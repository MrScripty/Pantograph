<script lang="ts">
  import { scale, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import {
    viewLevel,
    isAnimating,
    zoomTarget,
    animationConfig,
    type ViewLevel,
  } from '../stores/viewStore';

  interface Props {
    /** The current level this container represents */
    level: ViewLevel;
    /** Children to render inside the transition container */
    children?: import('svelte').Snippet;
  }

  let { level, children }: Props = $props();

  // Determine if this level should be visible
  let isVisible = $derived($viewLevel === level);

  // Animation parameters based on transition direction
  let transitionParams = $derived.by(() => {
    const config = $animationConfig;
    const target = $zoomTarget;

    // Default scale values
    let startScale = 0.1;
    let endScale = 1;

    // Adjust based on zoom direction
    if (level === 'orchestration') {
      // When zooming to orchestration, start small (zooming out)
      startScale = 2;
    } else if (level === 'data-graph') {
      // When zooming to data-graph, start large (zooming in)
      startScale = 0.1;
    } else if (level === 'group') {
      // Group zoom is similar to data-graph
      startScale = 0.1;
    }

    return {
      duration: config.duration,
      easing: cubicOut,
      start: startScale,
      opacity: 0,
    };
  });

  // Compute origin for zoom animation based on target node position
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

  /* Ensure smooth hardware-accelerated animations */
  .zoom-content {
    will-change: transform, opacity;
    backface-visibility: hidden;
  }

  /* Add subtle blur during zoom for depth effect */
  .animating .zoom-content {
    filter: blur(0px);
    transition: filter var(--animation-duration) ease-out;
  }
</style>
