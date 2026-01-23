<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import {
    breadcrumb,
    viewLevel,
    canNavigateBack,
    navigateBack,
    navigateToBreadcrumb,
    isAnimating,
    type BreadcrumbItem,
  } from '../stores/viewStore';

  // Icons for different view levels
  const levelIcons: Record<string, string> = {
    orchestration: '⚙️',
    'data-graph': '📊',
    group: '📦',
  };

  // Labels for view levels
  const levelLabels: Record<string, string> = {
    orchestration: 'Orchestration',
    'data-graph': 'Data Graph',
    group: 'Group',
  };

  function handleBreadcrumbClick(item: BreadcrumbItem, index: number) {
    // Don't navigate to the current (last) item
    if (index === $breadcrumb.length - 1) return;
    navigateToBreadcrumb(item);
  }

  function handleKeyDown(event: KeyboardEvent, item: BreadcrumbItem, index: number) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleBreadcrumbClick(item, index);
    }
  }
</script>

<nav
  class="navigation-breadcrumb"
  class:animating={$isAnimating}
  aria-label="Graph navigation"
>
  <!-- Back button -->
  {#if $canNavigateBack}
    <button
      class="back-button"
      onclick={() => navigateBack()}
      aria-label="Navigate back"
      transition:fade={{ duration: 150 }}
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M19 12H5M12 19l-7-7 7-7" />
      </svg>
    </button>
  {/if}

  <!-- Breadcrumb items -->
  <ol class="breadcrumb-list">
    {#each $breadcrumb as item, index (item.id)}
      <li
        class="breadcrumb-item"
        in:fly={{ x: -10, duration: 200, delay: index * 50 }}
      >
        {#if index > 0}
          <span class="separator" aria-hidden="true">
            <svg
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M9 18l6-6-6-6" />
            </svg>
          </span>
        {/if}

        <button
          class="breadcrumb-button"
          class:current={index === $breadcrumb.length - 1}
          onclick={() => handleBreadcrumbClick(item, index)}
          onkeydown={(e) => handleKeyDown(e, item, index)}
          aria-current={index === $breadcrumb.length - 1 ? 'page' : undefined}
          disabled={index === $breadcrumb.length - 1}
        >
          <span class="level-icon" aria-hidden="true">
            {levelIcons[item.level] || '📄'}
          </span>
          <span class="item-name">{item.name}</span>
          <span class="level-badge">{levelLabels[item.level]}</span>
        </button>
      </li>
    {/each}
  </ol>

  <!-- Current level indicator (when no breadcrumb items) -->
  {#if $breadcrumb.length === 0}
    <div class="current-level" transition:fade={{ duration: 150 }}>
      <span class="level-icon">{levelIcons[$viewLevel] || '📄'}</span>
      <span class="level-label">{levelLabels[$viewLevel] || 'Unknown'}</span>
    </div>
  {/if}
</nav>

<style>
  .navigation-breadcrumb {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(23, 23, 23, 0.9);
    backdrop-filter: blur(8px);
    border: 1px solid #404040;
    border-radius: 8px;
    font-size: 13px;
    user-select: none;
  }

  .animating {
    pointer-events: none;
    opacity: 0.7;
  }

  .back-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 1px solid #525252;
    border-radius: 6px;
    color: #a3a3a3;
    cursor: pointer;
    transition: all 150ms ease;
  }

  .back-button:hover {
    background: #262626;
    border-color: #737373;
    color: #e5e5e5;
  }

  .back-button:active {
    transform: scale(0.95);
  }

  .breadcrumb-list {
    display: flex;
    align-items: center;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .breadcrumb-item {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .separator {
    display: flex;
    align-items: center;
    color: #525252;
  }

  .breadcrumb-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: #a3a3a3;
    cursor: pointer;
    transition: all 150ms ease;
  }

  .breadcrumb-button:not(:disabled):hover {
    background: #262626;
    color: #e5e5e5;
  }

  .breadcrumb-button:disabled {
    cursor: default;
  }

  .breadcrumb-button.current {
    color: #e5e5e5;
    font-weight: 500;
  }

  .level-icon {
    font-size: 14px;
    line-height: 1;
  }

  .item-name {
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .level-badge {
    padding: 2px 6px;
    background: #262626;
    border-radius: 4px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #737373;
  }

  .breadcrumb-button.current .level-badge {
    background: #3730a3;
    color: #c7d2fe;
  }

  .current-level {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #a3a3a3;
  }

  .level-label {
    font-weight: 500;
    color: #e5e5e5;
  }
</style>
