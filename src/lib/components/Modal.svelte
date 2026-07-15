<script lang="ts">
    import type { Snippet } from "svelte";

    export interface ModalAction {
        label: string;
        kind?: "default" | "primary" | "danger";
        disabled?: boolean;
        onclick: () => void | Promise<void>;
    }

    interface Props {
        open: boolean;
        title: string;
        role?: "dialog" | "alertdialog";
        actions?: ModalAction[];
        onclose?: () => void;
        children?: Snippet;
    }

    let {
        open,
        title,
        role = "dialog",
        actions = [],
        onclose,
        children,
    }: Props = $props();

    function handleKeydown(event: KeyboardEvent) {
        if (open && event.key === "Escape" && onclose) {
            event.preventDefault();
            onclose();
        }
    }

    function focusModal(node: HTMLElement) {
        const previous = document.activeElement;
        queueMicrotask(() => {
            const target = node.querySelector<HTMLElement>(
                "textarea, input, select, button:not(:disabled), [tabindex]:not([tabindex='-1'])",
            );
            (target ?? node).focus();
        });

        return {
            destroy() {
                if (previous instanceof HTMLElement && previous.isConnected) {
                    previous.focus();
                }
            },
        };
    }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
    <div class="modal-overlay">
        {#if onclose}
            <button
                type="button"
                class="backdrop-dismiss"
                tabindex="-1"
                onclick={onclose}
                aria-label={`关闭${title}`}
            ></button>
        {/if}
        <div
            class="modal"
            {role}
            aria-modal="true"
            aria-label={title}
            tabindex="-1"
            use:focusModal
        >
            <h3>{title}</h3>
            {#if children}
                <div class="modal-body">{@render children()}</div>
            {/if}
            {#if actions.length > 0}
                <div class="modal-actions">
                    {#each actions as action}
                        <button
                            type="button"
                            class:primary={action.kind === "primary"}
                            class:danger={action.kind === "danger"}
                            disabled={action.disabled}
                            onclick={() => action.onclick()}
                        >
                            {action.label}
                        </button>
                    {/each}
                </div>
            {/if}
        </div>
    </div>
{/if}

<style>
    .modal-overlay {
        position: fixed;
        inset: 0;
        z-index: 50;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 16px;
        background: rgba(0, 0, 0, 0.3);
    }

    .backdrop-dismiss {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        padding: 0;
        cursor: default;
        background: transparent;
        border: 0;
    }

    .modal {
        position: relative;
        width: min(92vw, 360px);
        max-width: 100%;
        padding: 18px;
        color: var(--text-primary);
        background: var(--bg-primary);
        border: 1px solid var(--glass-border);
        border-radius: 14px;
        box-shadow: 0 16px 48px var(--glass-shadow);
    }

    h3 {
        margin: 0;
        font-size: 15px;
    }

    .modal-body {
        margin-top: 8px;
    }

    .modal-body :global(p) {
        margin: 0;
        color: var(--text-tertiary);
        font-size: 13px;
        line-height: 1.5;
    }

    .modal-body :global(p + p) {
        margin-top: 6px;
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        margin-top: 14px;
    }

    .modal-actions button {
        height: 34px;
        padding: 0 14px;
        color: var(--text-primary);
        font-size: 13px;
        font-weight: 500;
        cursor: pointer;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        transition: all 0.2s ease;
    }

    .modal-actions button:hover:not(:disabled) {
        background: var(--bg-hover);
        transform: translateY(-1px);
    }

    .modal-actions button.primary,
    .modal-actions button.danger {
        color: #fff;
    }

    .modal-actions button.primary {
        background: var(--accent-color);
        border-color: var(--accent-color);
    }

    .modal-actions button.danger {
        background: var(--danger-color);
        border-color: var(--danger-color);
    }

    .modal-actions button.primary:hover:not(:disabled),
    .modal-actions button.danger:hover:not(:disabled) {
        filter: brightness(1.08);
        box-shadow: 0 6px 14px rgba(0, 0, 0, 0.12);
    }

    .modal-actions button:active:not(:disabled) {
        box-shadow: none;
        transform: scale(0.96);
    }

    .modal-actions button:disabled {
        cursor: not-allowed;
        opacity: 0.55;
    }
</style>
