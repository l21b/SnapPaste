<script lang="ts">
    interface Props {
        enabled?: boolean;
        apiUrl?: string;
        apiKey?: string;
        model?: string;
        prompt?: string;
        temperature?: number;
    }

    let {
        enabled = $bindable(false),
        apiUrl = $bindable(""),
        apiKey = $bindable(""),
        model = $bindable(""),
        prompt = $bindable(""),
        temperature = $bindable(0.3),
    }: Props = $props();
    let expanded = $state(false);
</script>

<section aria-label="AI 设置">
    <div class="field">
        <div class="toggle-row">
            <span class="field-label">AI 文字润色</span>
            <label class="switch">
                <input type="checkbox" bind:checked={enabled} />
                <span class="switch-slider"></span>
            </label>
        </div>
        <small>选中文字后按快捷键进行润色</small>
    </div>

    {#if enabled}
        <button
            type="button"
            class="ghost-btn"
            onclick={() => (expanded = !expanded)}
        >
            {expanded ? "收起设置" : "展开设置"}
        </button>

        {#if expanded}
            <div class="ai-settings">
                <div class="field">
                    <span class="field-label">API 端点</span>
                    <input
                        type="text"
                        bind:value={apiUrl}
                        placeholder="https://api.openai.com/v1/chat/completions"
                    />
                    <small>支持兼容 OpenAI API 的服务端点</small>
                </div>
                <div class="field">
                    <span class="field-label">API Key</span>
                    <input type="password" bind:value={apiKey} placeholder="sk-..." />
                </div>
                <div class="field">
                    <span class="field-label">模型</span>
                    <input type="text" bind:value={model} placeholder="gpt-3.5-turbo" />
                </div>
                <div class="field">
                    <span class="field-label">提示词</span>
                    <textarea bind:value={prompt} rows="2"></textarea>
                </div>
                <div class="field">
                    <span class="field-label">Temperature ({temperature})</span>
                    <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.1"
                        bind:value={temperature}
                    />
                    <span class="hint">0 = 稳定输出, 1 = 创意随机</span>
                </div>
            </div>
        {/if}
    {/if}
</section>

<style>
    section {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
</style>
