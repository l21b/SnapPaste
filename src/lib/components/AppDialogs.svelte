<script lang="ts">
    import Modal from "./Modal.svelte";

    interface Props {
        addFavoriteOpen: boolean;
        favoriteInput?: string;
        addFavoriteSaving: boolean;
        aboutOpen: boolean;
        appVersion: string;
        clearOpen: boolean;
        favoritesOnly: boolean;
        hotkeyErrorOpen: boolean;
        hotkeyErrorMessage: string;
        hotkey: string;
        onCloseAddFavorite: () => void;
        onSubmitFavorite: () => void | Promise<void>;
        onCloseAbout: () => void;
        onCancelClear: () => void;
        onConfirmClear: () => void | Promise<void>;
        onCloseHotkeyError: () => void;
    }

    let {
        addFavoriteOpen,
        favoriteInput = $bindable(""),
        addFavoriteSaving,
        aboutOpen,
        appVersion,
        clearOpen,
        favoritesOnly,
        hotkeyErrorOpen,
        hotkeyErrorMessage,
        hotkey,
        onCloseAddFavorite,
        onSubmitFavorite,
        onCloseAbout,
        onCancelClear,
        onConfirmClear,
        onCloseHotkeyError,
    }: Props = $props();

    function handleFavoriteKeydown(event: KeyboardEvent) {
        if (
            event.key === "Enter" &&
            (event.ctrlKey || event.metaKey) &&
            favoriteInput.trim() &&
            !addFavoriteSaving
        ) {
            event.preventDefault();
            void onSubmitFavorite();
        }
    }
</script>

<Modal
    open={addFavoriteOpen}
    title="添加收藏"
    onclose={onCloseAddFavorite}
    actions={[
        {
            label: "取消",
            disabled: addFavoriteSaving,
            onclick: onCloseAddFavorite,
        },
        {
            label: addFavoriteSaving ? "保存中..." : "添加",
            kind: "primary",
            disabled: addFavoriteSaving || !favoriteInput.trim(),
            onclick: onSubmitFavorite,
        },
    ]}
>
    <textarea
        bind:value={favoriteInput}
        rows="4"
        placeholder="输入内容..."
        aria-label="收藏内容"
        onkeydown={handleFavoriteKeydown}
    ></textarea>
    <small>按 Ctrl+Enter 快速添加</small>
</Modal>

<Modal
    open={aboutOpen}
    title="关于 SnapPaste"
    onclose={onCloseAbout}
    actions={[
        { label: "知道了", kind: "primary", onclick: onCloseAbout },
    ]}
>
    <p>版本：v{appVersion}</p>
    <p>作者：21b</p>
</Modal>

<Modal
    open={clearOpen}
    title={favoritesOnly ? "清空收藏" : "清空历史"}
    role="alertdialog"
    onclose={onCancelClear}
    actions={[
        { label: "取消", onclick: onCancelClear },
        { label: "清空", kind: "danger", onclick: onConfirmClear },
    ]}
>
    <p>{favoritesOnly ? "将删除全部收藏项目" : "将删除全部历史记录"}</p>
</Modal>

<Modal
    open={hotkeyErrorOpen}
    title="快捷键注册失败"
    role="alertdialog"
    onclose={onCloseHotkeyError}
    actions={[
        { label: "确定", kind: "primary", onclick: onCloseHotkeyError },
    ]}
>
    <p>{hotkeyErrorMessage || `快捷键 ${hotkey} 注册失败。`}</p>
    <p>请关闭占用程序后重启应用，或在设置中更换快捷键。</p>
</Modal>

<style>
    textarea {
        width: 100%;
        min-height: 92px;
        padding: 8px 10px;
        color: var(--text-primary);
        font-size: 13px;
        resize: vertical;
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        outline: none;
    }

    textarea:focus {
        border-color: var(--accent-color);
    }

    small {
        display: block;
        margin-top: 6px;
        color: var(--text-tertiary);
        font-size: 11px;
    }
</style>
