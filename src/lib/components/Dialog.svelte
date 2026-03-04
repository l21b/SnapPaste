<!-- src/Dialog.svelte -->
<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";

  // 模式参数：'embedded'（嵌入主窗口）或 'popup'（独立窗口）
  let { mode = "embedded" }: { mode?: "embedded" | "popup" } = $props();

  // 弹窗状态
  let dialogType = $state<"info" | "error" | "success">("info");
  let dialogTitle = $state("");
  let dialogContent = $state("");
  let dialogVisible = $state(false);

  // 用于 popup 模式的自动关闭定时器
  let autoCloseTimer: ReturnType<typeof setTimeout> | undefined;

  // 关闭弹窗（根据模式决定行为）
  function closeDialog() {
    if (mode === "embedded") {
      dialogVisible = false;
    } else {
      // popup 模式：直接关闭窗口
      if (autoCloseTimer) clearTimeout(autoCloseTimer);
      getCurrentWindow().close();
    }
  }

  onMount(() => {
    // 监听 show-dialog 事件（来自后端）
    const unlisten1 = listen<{
      msg_type: string;
      title: string;
      content: string;
    }>("show-dialog", (event) => {
      dialogType = event.payload.msg_type as any;
      dialogTitle = event.payload.title;
      dialogContent = event.payload.content;
      dialogVisible = true;
    });

    // 监听 popup-content 事件（来自后端）
    const unlisten2 = listen<{
      msg_type: string;
      title: string;
      content: string;
    }>("popup-content", (event) => {
      dialogType = event.payload.msg_type as any;
      dialogTitle = event.payload.title;
      dialogContent = event.payload.content;
      dialogVisible = true;

      // popup 模式：3秒后自动关闭
      if (mode === "popup") {
        autoCloseTimer = setTimeout(() => {
          dialogVisible = false;
          setTimeout(() => {
            getCurrentWindow().close();
          }, 200);
        }, 3000);
      }
    });

    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
      if (autoCloseTimer) clearTimeout(autoCloseTimer);
    };
  });
</script>

{#if dialogVisible}
  <div class="dialog-overlay" role="dialog" aria-modal="true">
    <div class="dialog" class:dialog-error={dialogType === "error"} class:dialog-success={dialogType === "success"}>
      <div class="dialog-header">
        <span class="dialog-icon">
          {#if dialogType === "error"}❌
          {:else if dialogType === "success"}✅
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
  /* 样式完全保留原来的即可，无需改动 */
  .dialog-overlay { /* ... */ }
  .dialog { /* ... */ }
  /* ... 其余样式 ... */
</style>