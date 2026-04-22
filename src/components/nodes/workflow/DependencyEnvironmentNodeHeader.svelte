<script lang="ts">
  let { label }: { label: string } = $props();
</script>

<div class="flex items-center gap-2">
  <div class="w-5 h-5 rounded bg-cyan-700 flex items-center justify-center flex-shrink-0">
    <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="2"
        d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13a7 7 0 1114 0v3a2 2 0 01-2 2H5a2 2 0 01-2-2v-3z"
      />
    </svg>
  </div>
  <span class="text-sm font-medium text-neutral-200">{label}</span>
</div>
