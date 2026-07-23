<script>
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import DropZone from './lib/components/DropZone.svelte';
  import ConversionQueue from './lib/components/ConversionQueue.svelte';
  import FormatSelector from './lib/components/FormatSelector.svelte';
  import Settings from './lib/components/Settings.svelte';
  import History from './lib/components/History.svelte';

  let currentView = 'convert';
  let selectedFiles = [];
  let targetFormat = 'png';
  let quality = 'high';
  let conversionResults = [];
  let isConverting = false;

  async function handleFiles(files) {
    selectedFiles = files;
  }

  async function startConversion() {
    if (selectedFiles.length === 0) return;
    isConverting = true;
    conversionResults = [];

    try {
      if (selectedFiles.length === 1) {
        const file = selectedFiles[0];
        const outputPath = file.replace(/\.[^.]+$/, `.${targetFormat}`);
        const result = await invoke('convert_file', {
          request: {
            input_path: file,
            output_path: outputPath,
            target_format: targetFormat,
            quality: quality,
          }
        });
        conversionResults = [result];
      } else {
        const outputDir = selectedFiles[0].replace(/[/\\][^/\\]+$/, '');
        const results = await invoke('batch_convert', {
          request: {
            input_paths: selectedFiles,
            output_dir: outputDir,
            target_format: targetFormat,
            quality: quality,
          }
        });
        conversionResults = results;
      }
    } catch (err) {
      conversionResults = [{ success: false, error: err.toString() }];
    } finally {
      isConverting = false;
    }
  }

  async function openFiles() {
    try {
      const files = await open({ multiple: true });
      if (files) {
        selectedFiles = Array.isArray(files) ? files : [files];
      }
    } catch (err) {
      console.error('Failed to open files:', err);
    }
  }
</script>

<main>
  <nav>
    <button class:active={currentView === 'convert'} on:click={() => currentView = 'convert'}>
      🔄 Convert
    </button>
    <button class:active={currentView === 'history'} on:click={() => currentView = 'history'}>
      📋 History
    </button>
    <button class:active={currentView === 'settings'} on:click={() => currentView = 'settings'}>
      ⚙️ Settings
    </button>
  </nav>

  {#if currentView === 'convert'}
    <div class="convert-view">
      <DropZone on:files={e => handleFiles(e.detail)} />

      {#if selectedFiles.length > 0}
        <div class="conversion-options">
          <FormatSelector bind:targetFormat bind:quality />

          <div class="file-list">
            <h3>Selected Files ({selectedFiles.length})</h3>
            {#each selectedFiles as file}
              <div class="file-item">{file.split(/[/\\]/).pop()}</div>
            {/each}
          </div>

          <button class="convert-btn" on:click={startConversion} disabled={isConverting}>
            {#if isConverting}
              Converting...
            {:else}
              Convert {selectedFiles.length} file{selectedFiles.length > 1 ? 's' : ''}
            {/if}
          </button>
        </div>
      {/if}

      {#if conversionResults.length > 0}
        <div class="results">
          <h3>Results</h3>
          {#each conversionResults as result}
            <div class="result-item" class:success={result.success} class:failed={!result.success}>
              {#if result.success}
                ✅ Converted successfully
                {#if result.bytes_written}
                  <span class="size">({(result.bytes_written / 1024).toFixed(1)} KB)</span>
                {/if}
                {#if result.duration_ms}
                  <span class="time">in {result.duration_ms}ms</span>
                {/if}
              {:else}
                ❌ Failed: {result.error}
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>

  {:else if currentView === 'history'}
    <History />

  {:else if currentView === 'settings'}
    <Settings />
  {/if}
</main>

<style>
  main {
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
  }

  nav {
    display: flex;
    gap: 8px;
    margin-bottom: 24px;
    border-bottom: 1px solid #e0e0e0;
    padding-bottom: 12px;
  }

  nav button {
    padding: 8px 16px;
    border: none;
    background: transparent;
    cursor: pointer;
    border-radius: 6px;
    font-size: 14px;
    color: #666;
    transition: all 0.2s;
  }

  nav button:hover {
    background: #f0f0f0;
  }

  nav button.active {
    background: #e3f2fd;
    color: #1976d2;
    font-weight: 500;
  }

  .convert-view {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .conversion-options {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .file-list {
    background: #f8f9fa;
    border-radius: 8px;
    padding: 12px;
  }

  .file-list h3 {
    font-size: 14px;
    color: #666;
    margin-bottom: 8px;
  }

  .file-item {
    padding: 4px 8px;
    font-size: 13px;
    color: #333;
    background: white;
    border-radius: 4px;
    margin-bottom: 4px;
  }

  .convert-btn {
    padding: 12px 24px;
    background: #1976d2;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 16px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .convert-btn:hover:not(:disabled) {
    background: #1565c0;
  }

  .convert-btn:disabled {
    background: #ccc;
    cursor: not-allowed;
  }

  .results {
    background: #f8f9fa;
    border-radius: 8px;
    padding: 16px;
  }

  .results h3 {
    font-size: 14px;
    color: #666;
    margin-bottom: 12px;
  }

  .result-item {
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 14px;
    margin-bottom: 8px;
  }

  .result-item.success {
    background: #e8f5e9;
    color: #2e7d32;
  }

  .result-item.failed {
    background: #ffebee;
    color: #c62828;
  }

  .size, .time {
    color: #666;
    font-size: 12px;
    margin-left: 8px;
  }
</style>
