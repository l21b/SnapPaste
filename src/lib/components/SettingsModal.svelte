<script lang="ts">
    import type { Settings } from "$lib/types";
    import { formatHotkey, parseHotkey } from "$lib/hotkey";
    import { createDefaultSettings } from "$lib/settings";
    import BasicSettings from "./settings/BasicSettings.svelte";
    import DataTransferSettings from "./settings/DataTransferSettings.svelte";

    interface Props {
        open: boolean;
        settings: Settings;
        onsave?: (settings: Settings) => Promise<void> | void;
        onopenimport?: () => void;
        onopenexport?: () => void;
        onclose?: () => void;
    }

    let { open, settings, onsave, onopenimport, onopenexport, onclose }: Props =
        $props();
    let saving = $state(false);
    let draft = $state<Settings>(createDefaultSettings());

    $effect(() => {
        if (!open) return;
        const parsed = parseHotkey(settings.hotkey);
        draft = {
            ...settings,
            hotkey: formatHotkey(parsed.modifier, parsed.key),
        };
    });

    function closeModal() {
        if (!saving) onclose?.();
    }

    function handleWindowKeydown(event: KeyboardEvent) {
        if (open && event.key === "Escape") closeModal();
    }

    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        if (!onsave || saving || !draft.hotkey.trim()) return;

        saving = true;
        try {
            await onsave({ ...draft });
        } finally {
            saving = false;
        }
    }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if open}
    <div class="settings-backdrop">
        <button
            type="button"
            class="backdrop-dismiss"
            tabindex="-1"
            onclick={closeModal}
            aria-label="关闭设置"
        ></button>
        <form
            class="settings-modal"
            aria-label="设置"
            onsubmit={handleSubmit}
        >
            <div class="modal-header">
                <h2>设置</h2>
                <button
                    type="button"
                    class="icon-btn"
                    onclick={closeModal}
                    aria-label="关闭设置"
                >×</button>
            </div>

            <div class="modal-body">
                <BasicSettings
                    bind:hotkey={draft.hotkey}
                    bind:theme={draft.theme}
                    bind:keepDays={draft.keep_days}
                    bind:maxRecords={draft.max_records}
                    bind:autoStart={draft.auto_start}
                />

                <DataTransferSettings
                    onimport={onopenimport}
                    onexport={onopenexport}
                />
            </div>

            <div class="modal-footer">
                <button type="button" class="ghost-btn" onclick={closeModal}>
                    取消
                </button>
                <button type="submit" class="primary-btn" disabled={saving}>
                    {saving ? "保存中..." : "保存"}
                </button>
            </div>
        </form>
    </div>
{/if}

<style>
    .settings-backdrop {
        position: fixed;
        inset: 0;
        z-index: 10;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 6px;
        background: rgba(0, 0, 0, 0.5);
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

    .settings-modal {
        position: relative;
        display: flex;
        flex-direction: column;
        width: min(92vw, 260px);
        min-width: 240px;
        max-width: 100%;
        height: min(380px, calc(100vh - 12px));
        min-height: 280px;
        overflow: hidden;
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        border-radius: 16px;
        box-shadow: 0 16px 48px rgba(0, 0, 0, 0.3);
    }

    .modal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 10px 12px;
        border-bottom: 1px solid var(--border-color);
    }

    h2 {
        margin: 0;
        color: var(--text-primary);
        font-size: 15px;
    }

    .icon-btn {
        width: 28px;
        height: 28px;
        color: var(--text-tertiary);
        font-size: 18px;
        line-height: 1;
        cursor: pointer;
        background: transparent;
        border: 0;
        border-radius: 6px;
    }

    .icon-btn:hover {
        background: var(--bg-hover);
        box-shadow: 0 6px 14px rgba(0, 0, 0, 0.12);
        transform: translateY(-1px) scale(1.05);
    }

    .modal-body {
        display: flex;
        flex: 1;
        flex-direction: column;
        gap: 8px;
        min-height: 0;
        padding: 10px 12px;
        overflow-x: hidden;
        overflow-y: auto;
    }

    .settings-modal :global(.field) {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .settings-modal :global(.field-label) {
        color: var(--text-primary);
        font-size: 12px;
    }

    .settings-modal :global(small),
    .settings-modal :global(.hint) {
        color: var(--text-tertiary);
        font-size: 10px;
        white-space: normal;
        overflow-wrap: anywhere;
    }

    .settings-modal :global(input),
    .settings-modal :global(select) {
        width: 100%;
        min-width: 0;
        height: 32px;
        padding: 0 8px;
        color: var(--text-primary);
        font-size: 12px;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        outline: none;
    }

    .settings-modal :global(select option) {
        color: var(--text-primary);
        background: var(--bg-primary);
    }

    .settings-modal :global(input:focus),
    .settings-modal :global(select:focus),
    .settings-modal :global(textarea:focus) {
        border-color: var(--accent-color);
        outline: none;
    }

    .settings-modal :global(.hotkey-row),
    .settings-modal :global(.transfer-row) {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 6px;
    }

    .settings-modal :global(.hotkey-input) {
        text-align: center;
        letter-spacing: 0.2px;
    }

    .settings-modal :global(.ai-settings) {
        padding: 10px;
        margin-top: 4px;
        background: var(--bg-secondary);
        border-radius: 8px;
    }

    .settings-modal :global(.ai-settings .field + .field) {
        margin-top: 10px;
    }

    .settings-modal :global(.ai-settings textarea) {
        width: 100%;
        padding: 8px;
        color: var(--text-primary);
        font-family: inherit;
        font-size: 12px;
        resize: vertical;
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
    }

    .settings-modal :global(.toggle-row) {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
    }

    .settings-modal :global(.switch) {
        position: relative;
        display: inline-flex;
        width: 42px;
        height: 24px;
    }

    .settings-modal :global(.switch input) {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        margin: 0;
        cursor: pointer;
        opacity: 0;
    }

    .settings-modal :global(.switch-slider) {
        width: 100%;
        height: 100%;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 999px;
        transition: background-color 0.16s, border-color 0.16s;
    }

    .settings-modal :global(.switch-slider::after) {
        position: absolute;
        top: 3px;
        left: 3px;
        width: 16px;
        height: 16px;
        content: "";
        background: #fff;
        border-radius: 50%;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.28);
        transition: transform 0.16s;
    }

    .settings-modal :global(.switch input:checked + .switch-slider) {
        background: var(--accent-color);
        border-color: var(--accent-color);
    }

    .settings-modal :global(.switch input:checked + .switch-slider::after) {
        transform: translateX(18px);
    }

    .modal-footer {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        padding: 10px 12px;
        border-top: 1px solid var(--border-color);
    }

    .settings-modal :global(.ghost-btn),
    .primary-btn {
        height: 32px;
        padding: 0 12px;
        color: var(--text-primary);
        font-size: 13px;
        cursor: pointer;
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        transition: transform 0.16s, filter 0.16s, box-shadow 0.16s;
    }

    .primary-btn {
        color: #fff;
        background: var(--accent-color);
        border-color: var(--accent-color);
    }

    .settings-modal :global(.ghost-btn:hover),
    .primary-btn:hover {
        background-color: var(--bg-hover);
        box-shadow: 0 6px 14px rgba(0, 0, 0, 0.12);
        transform: translateY(-1px);
    }

    .primary-btn:hover {
        background-color: var(--accent-color);
        filter: brightness(1.08);
    }

    .icon-btn:active,
    .settings-modal :global(.ghost-btn:active),
    .primary-btn:active {
        box-shadow: none;
        transform: scale(0.96);
    }

    .settings-modal :global(.ghost-btn:disabled),
    .primary-btn:disabled {
        cursor: not-allowed;
        opacity: 0.6;
    }
</style>
