<script lang="ts">
    import { toast } from "$lib/toast.svelte";
</script>

<div class="toast-host" aria-live="polite" aria-atomic="false">
    {#each toast.messages as message (message.id)}
        <div
            class="toast"
            class:success={message.kind === "success"}
            class:error={message.kind === "error"}
            role={message.kind === "error" ? "alert" : "status"}
        >
            <span class="indicator" aria-hidden="true"></span>
            <span class="message">{message.text}</span>
            <button
                type="button"
                onclick={() => toast.remove(message.id)}
                aria-label="关闭提示"
            >×</button>
        </div>
    {/each}
</div>

<style>
    .toast-host {
        position: fixed;
        top: 12px;
        left: 50%;
        z-index: 200;
        display: flex;
        width: min(420px, calc(100vw - 24px));
        flex-direction: column;
        gap: 8px;
        pointer-events: none;
        transform: translateX(-50%);
    }

    .toast {
        display: grid;
        grid-template-columns: 6px minmax(0, 1fr) 24px;
        align-items: center;
        gap: 9px;
        min-height: 42px;
        padding: 8px 9px;
        color: var(--text-primary);
        font-size: 12px;
        line-height: 1.4;
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        border-radius: 10px;
        box-shadow: 0 10px 30px var(--glass-shadow);
        pointer-events: auto;
    }

    .indicator {
        width: 6px;
        height: 22px;
        background: var(--accent-color);
        border-radius: 999px;
    }

    .success .indicator {
        background: #22c55e;
    }

    .error .indicator {
        background: var(--danger-color);
    }

    .message {
        overflow-wrap: anywhere;
    }

    button {
        width: 24px;
        height: 24px;
        padding: 0;
        color: var(--text-tertiary);
        font-size: 18px;
        line-height: 1;
        cursor: pointer;
        background: transparent;
        border: 0;
        border-radius: 6px;
    }

    button:hover {
        color: var(--text-primary);
        background: var(--bg-hover);
    }
</style>
