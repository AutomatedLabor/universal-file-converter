<script>
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let history = [];
  let loading = true;

  onMount(async () => {
    await loadHistory();
  });

  async function loadHistory() {
    loading = true;
    try {
      history = await invoke('get_history');
    } catch (err) {
      console.error('Failed to load history:', err);
    } finally {
      loading = false;
    }
  }

  async function clearHistory() {
    try {
      await invoke('clear_history');
      history = [];
    } catch (err) {
      console.error('Failed to clear history:', err);
    }
  }

  function formatSize(bytes) {
    if (!bytes) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatTime(ms) {
    if (!ms) return '';
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
  }
</script>

<div class="history">
  <div class="header">
    <h2>Conversion History</h2>
    {#if history.length > 0}
      <button class="clear-btn" on:click={clearHistory}>Clear All</button>
    {/if}
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if history.length === 0}
    <div class="empty">No conversion history yet</div>
  {:else}
    <div class="list">
      {#each history as entry}
        <div class="entry" class:success={entry.success} class:failed={!entry.success}>
          <div class="status">{entry.success ? '✅' : '❌'}</div>
          <div class="details">
            <div class="paths">
              <span class="input">{entry.input_path.split(/[/\\]/).pop()}</span>
              <span class="arrow">→</span>
              <span class="output">{entry.output_path.split(/[/\\]/).pop()}</span>
            </div>
            <div class="meta">
              {entry.source_format} → {entry.target_format}
              {#if entry.bytes_written}
                · {formatSize(entry.bytes_written)}
              {/if}
              {#if entry.duration_ms}
                · {formatTime(entry.duration_ms)}
              {/if}
            </div>
            {#if entry.error}
              <div class="error">{entry.error}</div>
            {/if}
          </div>
          <div class="time">{new Date(entry.timestamp).toLocaleString()}</div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  h2 {
    font-size: 20px;
    color: #333;
  }

  .clear-btn {
    padding: 6px 12px;
    background: transparent;
    border: 1px solid #ddd;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    color: #666;
  }

  .clear-btn:hover {
    background: #f5f5f5;
  }

  .loading, .empty {
    text-align: center;
    padding: 40px;
    color: #999;
  }

  .entry {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px;
    border-radius: 8px;
    margin-bottom: 8px;
    background: #f8f9fa;
  }

  .entry.success { border-left: 3px solid #2e7d32; }
  .entry.failed { border-left: 3px solid #c62828; }

  .status { font-size: 18px; }

  .details { flex: 1; }

  .paths {
    font-size: 14px;
    margin-bottom: 4px;
  }

  .arrow {
    color: #999;
    margin: 0 6px;
  }

  .meta {
    font-size: 12px;
    color: #888;
  }

  .error {
    font-size: 12px;
    color: #c62828;
    margin-top: 4px;
  }

  .time {
    font-size: 12px;
    color: #999;
    white-space: nowrap;
  }
</style>
