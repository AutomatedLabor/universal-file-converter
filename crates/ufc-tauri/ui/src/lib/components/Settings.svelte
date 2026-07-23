<script>
  let concurrent = 4;
  let verifyOutput = true;
  let overwriteExisting = false;
  let darkMode = false;
</script>

<div class="settings">
  <h2>Settings</h2>

  <div class="section">
    <h3>Conversion</h3>
    <div class="field">
      <label for="concurrent">Max Concurrent Conversions</label>
      <input id="concurrent" type="number" bind:value={concurrent} min="1" max="16" />
    </div>
    <div class="field">
      <label>
        <input type="checkbox" bind:checked={verifyOutput} />
        Verify output integrity (checksum)
      </label>
    </div>
    <div class="field">
      <label>
        <input type="checkbox" bind:checked={overwriteExisting} />
        Overwrite existing files
      </label>
    </div>
  </div>

  <div class="section">
    <h3>Appearance</h3>
    <div class="field">
      <label>
        <input type="checkbox" bind:checked={darkMode} />
        Dark mode
      </label>
    </div>
  </div>
</div>

<style>
  .settings {
    max-width: 500px;
  }

  h2 {
    font-size: 20px;
    margin-bottom: 24px;
    color: #333;
  }

  .section {
    margin-bottom: 24px;
  }

  h3 {
    font-size: 14px;
    color: #666;
    margin-bottom: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .field {
    margin-bottom: 12px;
  }

  label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: #333;
    cursor: pointer;
  }

  input[type="number"] {
    width: 80px;
    padding: 8px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
    font-size: 14px;
  }

  input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }
</style>
