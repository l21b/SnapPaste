<script lang="ts">
    import type { Settings } from "$lib/types";
    import {
        formatHotkey,
        keyLabel,
        keyTokenFromEvent,
        MODIFIER_OPTIONS,
        parseHotkey,
    } from "$lib/hotkey";

    interface Props {
        hotkey?: string;
        theme?: Settings["theme"];
        keepDays?: number;
        maxRecords?: number;
        autoStart?: boolean;
    }

    let {
        hotkey = $bindable("Alt+Z"),
        theme = $bindable("system"),
        keepDays = $bindable(1),
        maxRecords = $bindable(500),
        autoStart = $bindable(false),
    }: Props = $props();

    let modifier = $state("Alt");
    let key = $state("Z");

    $effect(() => {
        const parsed = parseHotkey(hotkey);
        modifier = parsed.modifier;
        key = parsed.key;
    });

    function updateHotkey() {
        hotkey = formatHotkey(modifier, key);
    }

    function handleModifierChange(event: Event) {
        modifier = (event.target as HTMLSelectElement).value;
        updateHotkey();
    }

    function handleKeydown(event: KeyboardEvent) {
        event.preventDefault();
        const token = keyTokenFromEvent(event);
        if (!token) return;
        key = token;
        updateHotkey();
    }
</script>

<section aria-label="基础设置">
    <div class="field">
        <span class="field-label">快捷键</span>
        <div class="hotkey-row">
            <select value={modifier} oninput={handleModifierChange}>
                {#each MODIFIER_OPTIONS as option}
                    <option value={option.value}>{option.label}</option>
                {/each}
            </select>
            <input
                type="text"
                class="hotkey-input"
                value={keyLabel(key)}
                placeholder="按下按键"
                readonly
                onkeydown={handleKeydown}
            />
        </div>
    </div>

    <div class="field">
        <span class="field-label">主题</span>
        <select bind:value={theme}>
            <option value="system">跟随系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
        </select>
    </div>

    <div class="field">
        <span class="field-label">记录保留天数</span>
        <input type="number" min="0" bind:value={keepDays} />
        <small>0 代表永久保存</small>
    </div>

    <div class="field">
        <span class="field-label">最大记录数</span>
        <input type="number" min="0" step="1" bind:value={maxRecords} />
        <small>0 代表无限制</small>
    </div>

    <div class="field">
        <div class="toggle-row">
            <span class="field-label">开机启动</span>
            <label class="switch">
                <input type="checkbox" bind:checked={autoStart} />
                <span class="switch-slider"></span>
            </label>
        </div>
        <small>保存后立即生效</small>
    </div>
</section>

<style>
    section {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
</style>
