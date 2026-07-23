<script>
  export let targetFormat = 'png';
  export let quality = 'high';

  const formatGroups = [
    { label: 'Images', formats: ['png', 'jpg', 'webp', 'bmp', 'tiff', 'gif', 'avif', 'ico'] },
    { label: 'Documents', formats: ['pdf', 'docx', 'html', 'md', 'txt'] },
    { label: 'Audio', formats: ['wav', 'flac', 'mp3', 'ogg', 'opus'] },
    { label: 'Video', formats: ['mp4', 'webm', 'mkv'] },
    { label: 'Archives', formats: ['zip', 'tar', 'gz'] },
    { label: 'Data', formats: ['csv', 'json', 'xml', 'yaml'] },
  ];

  const qualities = [
    { value: 'lossless', label: 'Lossless' },
    { value: 'high', label: 'High Quality' },
    { value: 'medium', label: 'Medium' },
    { value: 'low', label: 'Low (smaller files)' },
  ];
</script>

<div class="format-selector">
  <div class="field">
    <label for="format">Target Format</label>
    <select id="format" bind:value={targetFormat}>
      {#each formatGroups as group}
        <optgroup label={group.label}>
          {#each group.formats as fmt}
            <option value={fmt}>{fmt.toUpperCase()}</option>
          {/each}
        </optgroup>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="quality">Quality</label>
    <select id="quality" bind:value={quality}>
      {#each qualities as q}
        <option value={q.value}>{q.label}</option>
      {/each}
    </select>
  </div>
</div>

<style>
  .format-selector {
    display: flex;
    gap: 16px;
  }

  .field {
    flex: 1;
  }

  label {
    display: block;
    font-size: 13px;
    color: #666;
    margin-bottom: 6px;
    font-weight: 500;
  }

  select {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
    font-size: 14px;
    background: white;
    cursor: pointer;
  }

  select:focus {
    outline: none;
    border-color: #1976d2;
    box-shadow: 0 0 0 2px rgba(25, 118, 210, 0.1);
  }
</style>
