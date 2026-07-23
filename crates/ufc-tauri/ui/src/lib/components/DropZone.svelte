<script>
  import { createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';

  const dispatch = createEventDispatcher();
  let isDragOver = false;

  function handleDragOver(e) {
    e.preventDefault();
    isDragOver = true;
  }

  function handleDragLeave() {
    isDragOver = false;
  }

  function handleDrop(e) {
    e.preventDefault();
    isDragOver = false;
    const files = Array.from(e.dataTransfer.files).map(f => f.path);
    if (files.length > 0) {
      dispatch('files', files);
    }
  }

  async function openFilePicker() {
    try {
      const files = await open({ multiple: true });
      if (files) {
        dispatch('files', Array.isArray(files) ? files : [files]);
      }
    } catch (err) {
      console.error('Failed to open files:', err);
    }
  }
</script>

<div
  class="dropzone"
  class:dragover={isDragOver}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
  on:click={openFilePicker}
  role="button"
  tabindex="0"
>
  <div class="icon">📁</div>
  <div class="text">
    {#if isDragOver}
      Drop files here
    {:else}
      Drag & drop files here, or click to browse
    {/if}
  </div>
  <div class="hint">Supports images, documents, audio, video, archives, and more</div>
</div>

<style>
  .dropzone {
    border: 2px dashed #ccc;
    border-radius: 12px;
    padding: 48px 24px;
    text-align: center;
    cursor: pointer;
    transition: all 0.2s;
    background: #fafafa;
  }

  .dropzone:hover, .dropzone.dragover {
    border-color: #1976d2;
    background: #e3f2fd;
  }

  .icon {
    font-size: 48px;
    margin-bottom: 12px;
  }

  .text {
    font-size: 16px;
    color: #333;
    margin-bottom: 8px;
  }

  .hint {
    font-size: 13px;
    color: #888;
  }
</style>
