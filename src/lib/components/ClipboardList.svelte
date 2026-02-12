<script lang="ts">
    import type { ClipboardRecord } from '$lib/types';
    import ClipboardItem from './ClipboardItem.svelte';

    interface Props {
        records: ClipboardRecord[];
        loading: boolean;
        oncopy?: (id: number) => void;
        ondelete?: (id: number) => void;
        onfavorite?: (id: number, favorite: boolean) => void;
        onpin?: (id: number, pinned: boolean) => void;
        emptyTitle?: string;
        emptyHint?: string;
    }

    let {
        records,
        loading,
        oncopy,
        ondelete,
        onfavorite,
        onpin,
        emptyTitle = '暂无历史记录',
        emptyHint = '复制内容后会自动记录'
    }: Props = $props();
</script>

<div class="clipboard-list">
    {#if loading}
        <div class="loading">
            <div class="spinner"></div>
            <span>加载中...</span>
        </div>
    {:else if records.length === 0}
        <div class="empty">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
            </svg>
            <p>{emptyTitle}</p>
            <span>{emptyHint}</span>
        </div>
    {:else}
        <div class="list-content">
            {#each records as record (record.id)}
                <ClipboardItem
                    {record}
                    {oncopy}
                    {ondelete}
                    {onfavorite}
                    {onpin}
                />
            {/each}
        </div>
    {/if}
</div>

<style>
    .clipboard-list {
        flex: 1;
        height: 100%;
        min-height: 0;
        overflow-y: auto;
        overflow-x: hidden;
        background: var(--bg-primary);
    }

    .loading,
    .empty {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 48px 24px;
        color: var(--text-secondary);
        text-align: center;
    }

    .empty svg {
        width: 52px;
        height: 52px;
        margin-bottom: 16px;
        opacity: 0.4;
    }

    .empty p {
        margin: 0 0 6px 0;
        font-size: 14px;
        font-weight: 500;
        color: var(--text-primary);
    }

    .loading span,
    .empty span {
        font-size: 12px;
        color: var(--text-tertiary);
    }

    .spinner {
        width: 28px;
        height: 28px;
        border: 2.5px solid var(--border-color);
        border-top-color: var(--accent-color);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
        margin-bottom: 14px;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .list-content {
        padding: 0;
    }
</style>
