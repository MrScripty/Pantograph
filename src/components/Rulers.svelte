<script lang="ts">
  import { onMount } from 'svelte';
  import { RULER_THICKNESS } from '../constants';
  import { panelWidth } from '../stores/panelStore';

  type Mark = {
    pos: number;
    isMajor: boolean;
    isNumbered: boolean;
  };

  let mousePos = $state({ x: 0, y: 0 });
  let windowSize = $state({ w: 0, h: 0 });

  const buildMarks = (length: number): Mark[] => {
    const marks: Mark[] = [];
    for (let i = 0; i < length; i += 10) {
      marks.push({
        pos: i,
        isMajor: i % 50 === 0,
        isNumbered: i % 100 === 0,
      });
    }
    return marks;
  };

  let marksH: Mark[] = $derived(buildMarks(windowSize.w));
  let marksV: Mark[] = $derived(buildMarks(windowSize.h));

  onMount(() => {
    const handleMouseMove = (e: MouseEvent) => {
      mousePos = { x: e.clientX, y: e.clientY };
    };
    const handleResize = () => {
      windowSize = { w: window.innerWidth, h: window.innerHeight };
    };

    handleResize();
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('resize', handleResize);
    };
  });
</script>

<div
  class="fixed top-0 left-0 bg-neutral-900 border-b border-neutral-800 z-50 overflow-hidden transition-[right] duration-300 ease-out"
  style="height: {RULER_THICKNESS}px; right: {$panelWidth}px;"
>
  {#each marksH as mark (mark.pos)}
    <div
      class="absolute border-neutral-600 border-l"
      style="left: {mark.pos}px; top: 0; height: {mark.isMajor ? '100%' : '40%'};"
    >
      {#if mark.isNumbered}
        <span class="absolute text-[8px] text-neutral-500 select-none" style="left: 4px; top: 4px;">
          {mark.pos}
        </span>
      {/if}
    </div>
  {/each}
  <div
    class="absolute top-0 h-full w-[2px] bg-blue-500 transition-transform duration-75"
    style="transform: translateX({mousePos.x}px);"
  ></div>
</div>

<div
  class="fixed top-0 left-0 bottom-0 bg-neutral-900 border-r border-neutral-800 z-50 overflow-hidden"
  style="width: {RULER_THICKNESS}px;"
>
  {#each marksV as mark (mark.pos)}
    <div
      class="absolute border-neutral-600 border-t"
      style="left: 0; top: {mark.pos}px; width: {mark.isMajor ? '100%' : '40%'};"
    >
      {#if mark.isNumbered}
        <span class="absolute text-[8px] text-neutral-500 select-none" style="left: 4px; top: 2px;">
          {mark.pos}
        </span>
      {/if}
    </div>
  {/each}
  <div
    class="absolute left-0 w-full h-[2px] bg-blue-500 transition-transform duration-75"
    style="transform: translateY({mousePos.y}px);"
  ></div>
</div>
