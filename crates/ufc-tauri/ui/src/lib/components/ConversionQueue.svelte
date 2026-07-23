<script>
  export let items = [];
</script>

<div class="queue">
  <h3>Conversion Queue</h3>
  {#if items.length === 0}
    <div class="empty">No items in queue</div>
  {:else}
    {#each items as item}
      <div class="item" class:active={item.status === 'converting'} class:completed={item.status === 'completed'} class:failed={item.status === 'failed'}>
        <div class="name">{item.name}</div>
        <div class="status">{item.status}</div>
        {#if item.progress !== undefined}
          <div class="progress-bar">
            <div class="progress-fill" style="width: {item.progress}%"></div>
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .queue {
    background: #f8f9fa;
    border-radius: 8px;
    padding: 16px;
  }

  h3 {
    font-size: 14px;
    color: #666;
    margin-bottom: 12px;
  }

  .empty {
    color: #999;
    font-size: 13px;
    text-align: center;
    padding: 20px;
  }

  .item {
    padding: 8px 12px;
    border-radius: 6px;
    margin-bottom: 8px;
    background: white;
  }

  .item.active { border-left: 3px solid #1976d2; }
  .item.completed { border-left: 3px solid #2e7d32; }
  .item.failed { border-left: 3px solid #c62828; }

  .name {
    font-size: 13px;
    font-weight: 500;
  }

  .status {
    font-size: 12px;
    color: #888;
    margin-top: 2px;
  }

  .progress-bar {
    height: 4px;
    background: #e0e0e0;
    border-radius: 2px;
    margin-top: 6px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: #1976d2;
    transition: width 0.3s;
  }
</style>
