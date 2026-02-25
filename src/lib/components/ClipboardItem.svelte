<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import type { ClipboardRecord } from "$lib/types";

    interface Props {
        record: ClipboardRecord;
        oncopy?: (id: number) => void;
        ondelete?: (id: number) => void;
        onfavorite?: (id: number, favorite: boolean) => void;
        onpin?: (id: number, pinned: boolean) => void;
    }

    let { record, oncopy, ondelete, onfavorite, onpin }: Props = $props();

    function getTypeLabel(typeStr: string): string {
        const labels: Record<string, string> = {
            text: "文本",
            image: "图片",
            html: "HTML",
            link: "链接",
        };
        return labels[typeStr] || typeStr;
    }

    function handleCopy() {
        oncopy?.(record.id);
    }

    function handleDelete(e: Event) {
        e.stopPropagation();
        ondelete?.(record.id);
    }

    function handleFavorite(e: Event) {
        e.stopPropagation();
        onfavorite?.(record.id, !record.is_favorite);
    }

    function handlePin(e: Event) {
        e.stopPropagation();
        onpin?.(record.id, !record.is_pinned);
    }

    async function handleOpenLink(e: Event) {
        e.stopPropagation();
        if (record.content_type === "link") {
            try {
                await invoke("open_url", { url: record.content });
            } catch (error) {
                console.error("Failed to open link:", error);
            }
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Enter" || e.key === " ") {
            handleCopy();
        } else if (e.key === "Delete") {
            e.stopPropagation();
            ondelete?.(record.id);
        }
    }

    function truncateText(text: string, maxLength: number = 100): string {
        if (text.length <= maxLength) return text;
        return text.slice(0, maxLength) + "...";
    }
</script>

<div
    class="clipboard-item"
    role="button"
    tabindex="0"
    onclick={handleCopy}
    onkeydown={handleKeydown}
>
    <div class="item-header">
        <span class="item-type">{getTypeLabel(record.content_type)}</span>
        {#if record.source_app && record.source_app !== "Unknown"}
            <span class="item-source">{record.source_app}</span>
        {/if}
    </div>
    <div class="item-content">
        {#if record.content_type === "text" || record.content_type === "link"}
            <p class="text-content">{truncateText(record.content)}</p>
        {:else if record.content_type === "image"}
            <div class="image-placeholder">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <rect x="3" y="3" width="18" height="18" rx="2" />
                    <circle cx="8.5" cy="8.5" r="1.5" />
                    <path d="M21 15l-5-5L5 21" />
                </svg>
                <span>图片数据</span>
            </div>
        {:else}
            <p class="text-content">{truncateText(record.content)}</p>
        {/if}
    </div>
    <button class="delete-btn" onclick={handleDelete} aria-label="删除">
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
        >
            <path
                d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"
            />
        </svg>
    </button>
    <button
        class="pin-btn"
        class:active={record.is_pinned}
        onclick={handlePin}
        aria-label={record.is_pinned ? "取消置顶" : "置顶"}
    >
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
        >
            <path
                d="M12 17v5M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6a2 2 0 0 0-2-2h-2a2 2 0 0 0-2 2v4.76z"
            />
        </svg>
    </button>
    {#if record.content_type === "link"}
        <button
            class="link-btn"
            onclick={handleOpenLink}
            aria-label="在浏览器打开链接"
        >
            <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <circle cx="12" cy="12" r="10" />
                <path
                    d="M2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z"
                />
            </svg>
        </button>
    {/if}
    <button
        class="favorite-btn"
        class:active={record.is_favorite}
        onclick={handleFavorite}
        aria-label={record.is_favorite ? "取消收藏" : "收藏"}
    >
        <svg viewBox="0 0 24 24" stroke-width="2">
            <path
                d="M12 3l2.9 5.88 6.49.95-4.7 4.58 1.11 6.47L12 17.8l-5.8 3.08 1.1-6.47-4.7-4.58 6.5-.95z"
            />
        </svg>
    </button>
</div>

<style>
    .clipboard-item {
        position: relative;
        padding: 12px 16px 14px 16px;
        border-bottom: 1px solid var(--border-color);
        cursor: pointer;
        transition: all 0.15s ease;
    }

    .clipboard-item:hover {
        background: var(--bg-hover);
    }

    .clipboard-item:focus {
        outline: none;
        background: var(--bg-hover);
    }

    .item-header {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 6px;
        padding-right: 90px;
        font-size: 11px;
    }

    .item-type {
        padding: 3px 8px;
        background: var(--accent-light);
        color: var(--accent-color);
        border-radius: 6px;
        font-weight: 500;
        font-size: 10px;
        letter-spacing: 0.02em;
    }

    .item-source {
        color: var(--text-tertiary);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .item-content {
        font-size: 13px;
        color: var(--text-primary);
        line-height: 1.4;
        word-break: break-all;
    }

    .text-content {
        margin: 0;
    }

    .image-placeholder {
        display: flex;
        align-items: center;
        gap: 6px;
        color: var(--text-secondary);
    }

    .image-placeholder svg {
        width: 16px;
        height: 16px;
    }

    .delete-btn {
        position: absolute;
        right: 36px;
        top: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        padding: 0;
        border: none;
        background: transparent;
        cursor: pointer;
        border-radius: 6px;
        opacity: 0;
        transition:
            opacity 0.16s,
            background-color 0.16s,
            transform 0.16s;
    }

    .pin-btn {
        position: absolute;
        right: 66px;
        top: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        padding: 0;
        border: none;
        background: transparent;
        cursor: pointer;
        border-radius: 6px;
        opacity: 0;
        transition:
            opacity 0.16s,
            background-color 0.16s,
            color 0.16s,
            transform 0.16s;
        color: var(--text-tertiary);
    }

    .favorite-btn {
        position: absolute;
        right: 6px;
        top: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        padding: 0;
        border: none;
        background: transparent;
        cursor: pointer;
        border-radius: 6px;
        opacity: 0;
        transition:
            opacity 0.16s,
            background-color 0.16s,
            color 0.16s,
            transform 0.16s;
        color: var(--text-tertiary);
    }

    .favorite-btn svg {
        width: 16px;
        height: 16px;
        fill: transparent;
        stroke: currentColor;
    }

    .pin-btn svg {
        width: 16px;
        height: 16px;
        stroke: currentColor;
    }

    .pin-btn.active {
        opacity: 1;
        color: var(--accent-color);
    }

    .pin-btn.active svg {
        fill: currentColor;
    }

    .link-btn {
        position: absolute;
        right: 96px;
        top: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        padding: 0;
        border: none;
        background: transparent;
        cursor: pointer;
        border-radius: 6px;
        opacity: 0;
        transition:
            opacity 0.16s,
            background-color 0.16s,
            color 0.16s,
            transform 0.16s;
        color: var(--text-tertiary);
    }

    .link-btn svg {
        width: 16px;
        height: 16px;
    }

    .link-btn:hover {
        color: #22c55e;
        background: rgba(34, 197, 94, 0.15);
        transform: translateY(-1px) scale(1.05);
    }

    .favorite-btn.active {
        opacity: 1;
        color: #f59e0b;
    }

    .favorite-btn.active svg {
        fill: currentColor;
        stroke: currentColor;
    }

    .clipboard-item:hover .delete-btn,
    .clipboard-item:hover .pin-btn,
    .clipboard-item:hover .favorite-btn,
    .clipboard-item:hover .link-btn {
        opacity: 1;
    }

    .delete-btn:hover {
        background: var(--danger-light);
        transform: translateY(-1px) scale(1.05);
    }

    .delete-btn:hover svg {
        color: var(--danger-color);
    }

    .delete-btn svg {
        width: 16px;
        height: 16px;
        color: var(--text-tertiary);
    }

    .favorite-btn:hover {
        background: rgba(245, 158, 11, 0.14);
        color: #f59e0b;
        transform: translateY(-1px) scale(1.05);
    }

    .pin-btn:hover {
        background: var(--accent-light);
        color: var(--accent-color);
        transform: translateY(-1px) scale(1.05);
    }

    .delete-btn:active,
    .favorite-btn:active,
    .pin-btn:active {
        transform: scale(0.95);
    }
</style>
