<script lang="ts">
    import type { Settings } from '$lib/types';

    interface Props {
        open: boolean;
        settings: Settings;
        onsave?: (settings: Settings) => Promise<void> | void;
        onopenimport?: () => void;
        onopenexport?: () => void;
        onclose?: () => void;
    }

    let { open, settings, onsave, onopenimport, onopenexport, onclose }: Props = $props();
    let saving = $state(false);
    let hotkeyModifier = $state('Ctrl+Shift');
    let hotkeyKey = $state('V');
    let hotkeyError = $state('');

    const modifierOptions = [
        { value: '', label: '无修饰键' },
        { value: 'Ctrl', label: 'Ctrl' },
        { value: 'Alt', label: 'Alt' },
        { value: 'Shift', label: 'Shift' },
        { value: 'Ctrl+Alt', label: 'Ctrl + Alt' },
        { value: 'Ctrl+Shift', label: 'Ctrl + Shift' },
        { value: 'Alt+Shift', label: 'Alt + Shift' }
    ] as const;

    let draft = $state<Settings>({
        hotkey_modifiers: 0,
        hotkey_key: 0,
        hotkey: 'Ctrl+Shift+V',
        theme: 'system',
        keep_days: 1,
        max_records: 500,
        auto_start: false
    });

    function normalizeModifiers(values: string[]): string {
        const hasCtrl = values.includes('Ctrl');
        const hasAlt = values.includes('Alt');
        const hasShift = values.includes('Shift');
        const normalized = [
            hasCtrl ? 'Ctrl' : '',
            hasAlt ? 'Alt' : '',
            hasShift ? 'Shift' : ''
        ].filter(Boolean);
        return normalized.join('+');
    }

    function parseHotkey(hotkey: string): { modifier: string; key: string } {
        const tokens = hotkey
            .split('+')
            .map((t) => t.trim())
            .filter(Boolean);

        if (tokens.length === 0) {
            return { modifier: 'Ctrl+Shift', key: 'V' };
        }

        const key = normalizeKeyToken(tokens[tokens.length - 1]);
        const modTokens = tokens.slice(0, -1).map((t) => {
            const upper = t.toUpperCase();
            if (upper === 'CTRL' || upper === 'CONTROL') return 'Ctrl';
            if (upper === 'ALT' || upper === 'OPTION') return 'Alt';
            if (upper === 'SHIFT') return 'Shift';
            return '';
        }).filter(Boolean);

        const modifier = normalizeModifiers(modTokens);
        const hasOption = modifierOptions.some((o) => o.value === modifier);
        return {
            modifier: hasOption ? modifier : '',
            key: key || 'V'
        };
    }

    function normalizeKeyToken(raw: string): string {
        const token = raw.trim();
        if (!token) return '';

        if (/^[a-zA-Z0-9]$/.test(token)) {
            return token.toUpperCase();
        }

        const upper = token.toUpperCase();
        if (/^KEY[A-Z]$/.test(upper)) {
            return upper.slice(3);
        }
        if (/^DIGIT[0-9]$/.test(upper)) {
            return upper.slice(5);
        }
        if (/^F([1-9]|1[0-9]|2[0-4])$/.test(upper)) {
            return upper;
        }
        if (/^NUMPAD[0-9]$/.test(upper)) {
            return `Numpad${upper.slice(6)}`;
        }

        switch (upper) {
            case 'ESCAPE':
            case 'ESC':
                return 'Esc';
            case ' ':
            case 'SPACE':
            case 'SPACEBAR':
                return 'Space';
            case 'ARROWUP':
                return 'ArrowUp';
            case 'ARROWDOWN':
                return 'ArrowDown';
            case 'ARROWLEFT':
                return 'ArrowLeft';
            case 'ARROWRIGHT':
                return 'ArrowRight';
            default:
                return token;
        }
    }

    function keyLabel(token: string): string {
        if (/^Numpad[0-9]$/.test(token)) {
            return token.replace('Numpad', 'Num');
        }
        return token;
    }

    function formatHotkey(modifier: string, key: string): string {
        if (!modifier) return key;
        return `${modifier}+${key}`;
    }

    function keyTokenFromEvent(e: KeyboardEvent): string {
        if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
            return '';
        }

        if (/^Key[A-Z]$/.test(e.code)) {
            return e.code.slice(3);
        }

        if (/^Digit[0-9]$/.test(e.code)) {
            return e.code.slice(5);
        }

        if (/^Numpad[0-9]$/.test(e.code)) {
            return e.code;
        }

        if (/^F([1-9]|1[0-9]|2[0-4])$/i.test(e.code)) {
            return e.code.toUpperCase();
        }

        if (/^[a-zA-Z0-9]$/.test(e.key)) {
            return e.key.toUpperCase();
        }

        switch (e.key) {
            case 'ArrowUp': return 'ArrowUp';
            case 'ArrowDown': return 'ArrowDown';
            case 'ArrowLeft': return 'ArrowLeft';
            case 'ArrowRight': return 'ArrowRight';
            case 'Enter': return 'Enter';
            case 'Tab': return 'Tab';
            case 'Escape': return 'Esc';
            case 'Backspace': return 'Backspace';
            case 'Delete': return 'Delete';
            case 'Insert': return 'Insert';
            case 'Home': return 'Home';
            case 'End': return 'End';
            case 'PageUp': return 'PageUp';
            case 'PageDown': return 'PageDown';
            case ' ': return 'Space';
            default: return normalizeKeyToken(e.key);
        }
    }

    function updateDraftHotkey() {
        draft.hotkey = formatHotkey(hotkeyModifier, hotkeyKey);
    }

    function closeModal() {
        onclose?.();
    }

    function handleHotkeyModifierChange(e: Event) {
        hotkeyModifier = (e.target as HTMLSelectElement).value;
        hotkeyError = '';
        updateDraftHotkey();
    }

    function handleHotkeyKeydown(e: KeyboardEvent) {
        e.preventDefault();
        const token = keyTokenFromEvent(e);
        if (!token) return;
        hotkeyKey = token;
        hotkeyError = '';
        updateDraftHotkey();
    }

    $effect(() => {
        // 显式跟踪 open 和 settings 的变化
        if (open) {
            // 确保读取 settings 的所有属性以跟踪依赖
            const _theme = settings.theme;
            const _hotkey = settings.hotkey;
            const parsed = parseHotkey(_hotkey);
            draft = {
                ...settings,
                hotkey: formatHotkey(parsed.modifier, parsed.key)
            };
            hotkeyModifier = parsed.modifier;
            hotkeyKey = parsed.key;
            hotkeyError = '';
        }
    });

    async function handleSubmit(e: Event) {
        e.preventDefault();
        if (!onsave || saving) return;
        if (!hotkeyKey.trim()) {
            hotkeyError = '请按下一个主键';
            return;
        }

        updateDraftHotkey();
        saving = true;
        try {
            await onsave({ ...draft });
            closeModal();
        } finally {
            saving = false;
        }
    }

    function handleBackdropClick(e: MouseEvent) {
        if (e.target === e.currentTarget) {
            closeModal();
        }
    }

    function handleBackdropKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            closeModal();
        }
    }
</script>

{#if open}
    <div
        class="settings-backdrop"
        role="button"
        tabindex="0"
        onclick={handleBackdropClick}
        onkeydown={handleBackdropKeydown}
    >
        <form class="settings-modal" onsubmit={handleSubmit}>
            <div class="modal-header">
                <h2>设置</h2>
                <button type="button" class="icon-btn" onclick={closeModal} aria-label="关闭设置">
                    ×
                </button>
            </div>

            <div class="modal-body">
                <div class="field">
                    <span class="field-label">快捷键</span>
                    <div class="hotkey-row">
                        <select value={hotkeyModifier} oninput={handleHotkeyModifierChange}>
                            {#each modifierOptions as option}
                                <option value={option.value}>{option.label}</option>
                            {/each}
                        </select>
                        <input
                            type="text"
                            class="hotkey-input"
                            value={keyLabel(hotkeyKey)}
                            placeholder="按下按键"
                            readonly
                            onkeydown={handleHotkeyKeydown}
                        />
                    </div>
                    {#if hotkeyError}
                        <small class="error">{hotkeyError}</small>
                    {/if}
                </div>

                <div class="field">
                    <span class="field-label">主题</span>
                    <select bind:value={draft.theme}>
                        <option value="system">跟随系统</option>
                        <option value="light">浅色</option>
                        <option value="dark">深色</option>
                    </select>
                </div>

                <div class="field">
                    <span class="field-label">记录保留天数</span>
                    <input type="number" min="0" bind:value={draft.keep_days} />
                    <small>0 代表永久保存</small>
                </div>

                <div class="field">
                    <span class="field-label">最大记录数</span>
                    <input type="number" min="0" step="1" bind:value={draft.max_records} />
                    <small>0 代表无限制</small>
                </div>

                <div class="field">
                    <div class="toggle-row">
                        <span class="field-label">开机启动</span>
                        <label class="switch">
                            <input type="checkbox" bind:checked={draft.auto_start} />
                            <span class="switch-slider"></span>
                        </label>
                    </div>
                    <small>保存后立即生效</small>
                </div>

                <div class="field">
                    <span class="field-label">收藏导入导出</span>
                    <div class="transfer-row">
                        <button type="button" class="ghost-btn" onclick={() => onopenimport?.()}>
                            导入
                        </button>
                        <button type="button" class="ghost-btn" onclick={() => onopenexport?.()}>
                            导出
                        </button>
                    </div>
                    <small>文件格式为JSON,导入为增量模式</small>
                </div>

            </div>

            <div class="modal-footer">
                <button type="button" class="ghost-btn" onclick={closeModal}>取消</button>
                <button type="submit" class="primary-btn" disabled={saving}>
                    {saving ? '保存中...' : '保存'}
                </button>
            </div>
        </form>
    </div>
{/if}

<style>
    .settings-backdrop {
        position: fixed;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 6px;
        background: rgba(0, 0, 0, 0.5);
        z-index: 10;
    }

    .settings-modal {
        width: min(92vw, 260px);
        min-width: 240px;
        max-width: 100%;
        height: min(380px, calc(100vh - 12px));
        min-height: 280px;
        display: flex;
        flex-direction: column;
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        border-radius: 16px;
        box-shadow: 0 16px 48px rgba(0, 0, 0, 0.3);
        overflow: hidden;
    }

    .modal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 10px 12px;
        border-bottom: 1px solid var(--border-color);
    }

    .modal-header h2 {
        margin: 0;
        font-size: 15px;
        color: var(--text-primary);
    }

    .icon-btn {
        width: 28px;
        height: 28px;
        border: none;
        border-radius: 6px;
        background: transparent;
        color: var(--text-tertiary);
        cursor: pointer;
        font-size: 18px;
        line-height: 1;
        transition: background-color 0.16s, transform 0.16s, box-shadow 0.16s;
    }

    .icon-btn:hover {
        background: var(--bg-hover);
        transform: translateY(-1px) scale(1.05);
        box-shadow: 0 6px 14px rgba(0, 0, 0, 0.12);
    }

    .modal-body {
        flex: 1;
        min-height: 0;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 10px 12px;
        overflow-y: auto;
        overflow-x: hidden;
    }

    .field {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .field-label {
        font-size: 12px;
        color: var(--text-primary);
    }

    .field small {
        font-size: 10px;
        color: var(--text-tertiary);
        white-space: normal;
        overflow-wrap: anywhere;
    }

    .error {
        color: var(--danger-color);
    }

    input,
    select {
        width: 100%;
        min-width: 0;
        height: 32px;
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 0 8px;
        font-size: 12px;
        color: var(--text-primary);
        background: var(--bg-secondary);
        outline: none;
    }

    select option {
        background: var(--bg-primary);
        color: var(--text-primary);
    }

    input:focus,
    select:focus {
        border-color: var(--accent-color);
    }

    .hotkey-row {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 6px;
        align-items: center;
    }

    .hotkey-row select,
    .hotkey-row .hotkey-input {
        height: 30px;
        padding: 0 6px;
        font-size: 11px;
    }

    .hotkey-input {
        text-align: center;
        letter-spacing: 0.2px;
    }

    .transfer-row {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 6px;
    }

    .toggle-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
    }

    .switch {
        position: relative;
        width: 42px;
        height: 24px;
        display: inline-flex;
    }

    .switch input {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        margin: 0;
        opacity: 0;
        cursor: pointer;
    }

    .switch-slider {
        width: 100%;
        height: 100%;
        border-radius: 999px;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        transition: background-color 0.16s, border-color 0.16s;
    }

    .switch-slider::after {
        content: '';
        position: absolute;
        top: 3px;
        left: 3px;
        width: 16px;
        height: 16px;
        border-radius: 50%;
        background: #fff;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.28);
        transition: transform 0.16s;
    }

    .switch input:checked + .switch-slider {
        background: var(--accent-color);
        border-color: var(--accent-color);
    }

    .switch input:checked + .switch-slider::after {
        transform: translateX(18px);
    }

    .modal-footer {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        padding: 10px 12px;
        border-top: 1px solid var(--border-color);
    }

    .ghost-btn,
    .primary-btn {
        height: 32px;
        padding: 0 12px;
        border-radius: 8px;
        border: 1px solid var(--border-color);
        background: var(--bg-primary);
        color: var(--text-primary);
        cursor: pointer;
        font-size: 13px;
        transition: transform 0.16s, filter 0.16s, box-shadow 0.16s;
    }

    .primary-btn {
        border-color: var(--accent-color);
        background: var(--accent-color);
        color: #fff;
    }

    .ghost-btn:hover {
        transform: translateY(-1px);
        background: var(--bg-hover);
        color: var(--text-primary);
        box-shadow: 0 6px 14px rgba(0, 0, 0, 0.12);
    }

    .primary-btn:hover {
        transform: translateY(-1px);
        box-shadow: 0 6px 14px rgba(0, 0, 0, 0.12);
    }

    .icon-btn:active,
    .ghost-btn:active,
    .primary-btn:active {
        transform: scale(0.96);
        box-shadow: none;
    }

    .ghost-btn:disabled,
    .primary-btn:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
</style>
