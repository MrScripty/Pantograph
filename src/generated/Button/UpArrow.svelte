<script lang="ts">
let { label = "UP", iconColor = "text-green-500" } = $props();

let isHovered = $state(false);

const baseClasses = 'relative overflow-hidden transition-all duration-200';
const hoverClasses = 'scale-105 shadow-lg';

</script>

<button
  class={`w-full ${baseClasses} ${isHovered ? hoverClasses : ''}`}
  onmouseenter={() => isHovered = true}
  onmouseleave={() => isHovered = false}
>
  <div class="flex items-center justify-between px-4 py-3 bg-white border border-white/20 rounded-lg">
    <span class="text-blue-500 font-medium">{label}</span>
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke={iconColor}
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M18 15v4L12 13l-6 4V15"></path>
      <polyline points="6,9 12,3 18,9"></polyline>
    </svg>
  </div>
</button>