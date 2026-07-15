<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  let dialogType = $state<"info" | "error">("info");
  let dialogTitle = $state("");
  let dialogContent = $state("");
  let dialogVisible = $state(false);

  function closeDialog() {
    dialogVisible = false;
  }

  onMount(() => {
    const unlisten = listen<{
      msg_type: string;
      title: string;
      content: string;
    }>("popup-content", (event) => {
      dialogType = normalizeDialogType(event.payload.msg_type);
      dialogTitle = event.payload.title;
      dialogContent = event.payload.content;
      dialogVisible = true;
    });

    return () => {
      unlisten.then((dispose) => dispose());
    };
  });

  function normalizeDialogType(value: string): "info" | "error" {
    return value === "error" ? "error" : "info";
  }
</script>

{#if dialogVisible}
  <div class="dialog-overlay" role="dialog" aria-modal="true">
    <div class="dialog" class:dialog-error={dialogType === "error"}>
      <div class="dialog-header">
        <span class="dialog-icon">
          {#if dialogType === "error"}❌
          {:else}ℹ️{/if}
        </span>
        <h3 class="dialog-title">{dialogTitle}</h3>
      </div>
      <div class="dialog-content">
        <p>{dialogContent}</p>
      </div>
      <div class="dialog-footer">
        <button class="dialog-btn" onclick={closeDialog}>确定</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background: rgba(0, 0, 0, 0.35);
  }

  .dialog {
    width: min(92vw, 360px);
    padding: 18px;
    color: var(--text-primary);
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 14px;
    box-shadow: 0 16px 48px var(--glass-shadow);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dialog-icon {
    font-size: 18px;
  }

  .dialog-title {
    margin: 0;
    font-size: 15px;
  }

  .dialog-content {
    margin-top: 10px;
    color: var(--text-tertiary);
    font-size: 13px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }

  .dialog-content p {
    margin: 0;
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }

  .dialog-btn {
    height: 34px;
    padding: 0 14px;
    color: #fff;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    background: var(--accent-color);
    border: 1px solid var(--accent-color);
    border-radius: 8px;
  }

  .dialog-error .dialog-btn {
    background: var(--danger-color);
    border-color: var(--danger-color);
  }

  .dialog-btn:active {
    transform: scale(0.96);
  }
</style>
