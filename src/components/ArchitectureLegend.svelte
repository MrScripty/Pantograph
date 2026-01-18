<script lang="ts">
  import { CATEGORY_COLORS, CONNECTION_STYLES } from '../services/architecture/types';
  import type { ArchNodeCategory, ArchConnectionType } from '../services/architecture/types';

  const categories: { key: ArchNodeCategory; label: string }[] = [
    { key: 'component', label: 'Component' },
    { key: 'service', label: 'Service' },
    { key: 'store', label: 'Store' },
    { key: 'command', label: 'Command' },
    { key: 'backend', label: 'Backend' },
  ];

  const connectionTypes: { key: ArchConnectionType; label: string }[] = [
    { key: 'import', label: 'Import' },
    { key: 'command', label: 'Tauri Cmd' },
    { key: 'subscription', label: 'Store Sub' },
    { key: 'event', label: 'Event' },
    { key: 'uses', label: 'Uses' },
  ];
</script>

<div class="architecture-legend w-48 bg-neutral-900/95 border-r border-neutral-700 p-3 flex flex-col gap-4 overflow-y-auto">
  <!-- Node Categories -->
  <div>
    <h3 class="text-[10px] uppercase tracking-wider text-neutral-500 mb-2">Node Types</h3>
    <div class="flex flex-col gap-1.5">
      {#each categories as cat}
        <div class="flex items-center gap-2">
          <div
            class="w-3 h-3 rounded"
            style="background: {CATEGORY_COLORS[cat.key]};"
          ></div>
          <span class="text-xs text-neutral-400">{cat.label}</span>
        </div>
      {/each}
    </div>
  </div>

  <!-- Connection Types -->
  <div>
    <h3 class="text-[10px] uppercase tracking-wider text-neutral-500 mb-2">Connections</h3>
    <div class="flex flex-col gap-1.5">
      {#each connectionTypes as conn}
        <div class="flex items-center gap-2">
          <svg width="20" height="12" class="flex-shrink-0">
            <line
              x1="0"
              y1="6"
              x2="20"
              y2="6"
              stroke={CONNECTION_STYLES[conn.key].stroke}
              stroke-width="2"
              stroke-dasharray={CONNECTION_STYLES[conn.key].strokeDasharray}
            />
          </svg>
          <span class="text-xs text-neutral-400">{conn.label}</span>
        </div>
      {/each}
    </div>
  </div>

  <!-- Info -->
  <div class="mt-auto pt-4 border-t border-neutral-700/50">
    <p class="text-[10px] text-neutral-500 leading-relaxed">
      Click a node to see its file path in the console. Use filters above to show/hide node types and connections.
    </p>
  </div>
</div>
