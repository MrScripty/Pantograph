import { writable, derived } from 'svelte/store';
import type { Node, Edge } from '@xyflow/svelte';
import type { ArchNodeCategory, ArchConnectionType } from '../services/architecture/types';
import { CATEGORY_COLORS, CONNECTION_STYLES } from '../services/architecture/types';
import { PANTOGRAPH_ARCHITECTURE } from '../config/architecture';
import { layoutArchitecture } from '../services/architecture/layout';

// --- Filter State ---

export const visibleCategories = writable<Set<ArchNodeCategory>>(
  new Set(['component', 'service', 'store', 'backend', 'command'])
);

export const visibleConnectionTypes = writable<Set<ArchConnectionType>>(
  new Set(['import', 'command', 'subscription', 'event', 'uses'])
);

export const searchQuery = writable<string>('');

// --- Computed Positions ---

const positions = layoutArchitecture(PANTOGRAPH_ARCHITECTURE);

// --- Derived SvelteFlow Nodes ---

export const architectureNodes = derived(
  [visibleCategories, searchQuery],
  ([$visible, $query]) => {
    const queryLower = $query.toLowerCase();

    return PANTOGRAPH_ARCHITECTURE.nodes
      .filter((n) => $visible.has(n.category))
      .filter((n) =>
        $query === '' ||
        n.label.toLowerCase().includes(queryLower) ||
        (n.description?.toLowerCase().includes(queryLower) ?? false)
      )
      .map((n): Node => ({
        id: n.id,
        type: `arch-${n.category}`,
        position: positions[n.id] || { x: 0, y: 0 },
        data: {
          label: n.label,
          description: n.description,
          filePath: n.filePath,
          category: n.category,
          color: CATEGORY_COLORS[n.category],
        },
        draggable: true,
        selectable: true,
        connectable: false,
      }));
  }
);

// --- Derived SvelteFlow Edges ---

export const architectureEdges = derived(
  [visibleConnectionTypes, architectureNodes],
  ([$visible, $nodes]) => {
    const nodeIds = new Set($nodes.map((n) => n.id));

    return PANTOGRAPH_ARCHITECTURE.connections
      .filter((c) => $visible.has(c.connectionType))
      .filter((c) => nodeIds.has(c.source) && nodeIds.has(c.target))
      .map((c): Edge => {
        const style = CONNECTION_STYLES[c.connectionType];
        const strokeWidth = c.connectionType === 'uses' ? 1 : 2;
        return {
          id: c.id,
          source: c.source,
          target: c.target,
          type: 'smoothstep',
          animated: c.connectionType === 'event',
          style: `stroke: ${style.stroke}; stroke-dasharray: ${style.strokeDasharray}; stroke-width: ${strokeWidth}px;`,
          data: {
            connectionType: c.connectionType,
            label: c.label,
          },
        };
      });
  }
);

// --- Filter Actions ---

export function toggleCategory(category: ArchNodeCategory) {
  visibleCategories.update((set) => {
    const newSet = new Set(set);
    if (newSet.has(category)) {
      newSet.delete(category);
    } else {
      newSet.add(category);
    }
    return newSet;
  });
}

export function toggleConnectionType(connectionType: ArchConnectionType) {
  visibleConnectionTypes.update((set) => {
    const newSet = new Set(set);
    if (newSet.has(connectionType)) {
      newSet.delete(connectionType);
    } else {
      newSet.add(connectionType);
    }
    return newSet;
  });
}

export function showAllCategories() {
  visibleCategories.set(new Set(['component', 'service', 'store', 'backend', 'command']));
}

export function showAllConnectionTypes() {
  visibleConnectionTypes.set(new Set(['import', 'command', 'subscription', 'event', 'uses']));
}

export function clearSearch() {
  searchQuery.set('');
}
