<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { getVersion } from "@tauri-apps/api/app";
    import { open as openDialog } from "@tauri-apps/plugin-dialog";
    import { onMount } from "svelte";
    import type { Settings } from "$lib/types";
    import { ClipboardHistory } from "$lib/history.svelte";
    import { errorMessage } from "$lib/errors";
    import {
        createDefaultSettings,
        getHistoryQueryLimit,
    } from "$lib/settings";
    import { registerAppEvents } from "$lib/tauri-events";
    import { toast } from "$lib/toast.svelte";
    import SearchBar from "$lib/components/SearchBar.svelte";
    import ClipboardList from "$lib/components/ClipboardList.svelte";
    import SettingsModal from "$lib/components/SettingsModal.svelte";
    import AppDialogs from "$lib/components/AppDialogs.svelte";
    import Dialog from "$lib/components/Dialog.svelte";
    import ToastHost from "$lib/components/ToastHost.svelte";

    type ExportFavoritesResult = {
        count: number;
        path: string;
    };

    let settingsOpen = $state(false);
    let aboutOpen = $state(false);
    let appVersion = $state("0.1.0");
    let clearConfirmOpen = $state(false);
    let addFavoriteOpen = $state(false);
    let hotkeyErrorOpen = $state(false);
    let hotkeyErrorMessage = $state("");
    let favoriteInput = $state("");
    let addFavoriteSaving = $state(false);
    let searchBarRef: { focusInput: () => void } | null = null;
    let settings = $state<Settings>(createDefaultSettings());

    const history = new ClipboardHistory({
        getLimit: () => getHistoryQueryLimit(settings.max_records),
        isRefreshBlocked: () =>
            settingsOpen || clearConfirmOpen || addFavoriteOpen,
    });

    function preApplyCachedTheme() {
        if (typeof window === "undefined") return;
        const cached = window.localStorage.getItem("snappaste-theme");
        if (cached === "light" || cached === "dark") {
            document.documentElement.setAttribute("data-theme", cached);
        } else if (cached === "system") {
            document.documentElement.removeAttribute("data-theme");
        }
    }

    preApplyCachedTheme();

    function applyTheme(theme: Settings["theme"]) {
        const root = document.documentElement;
        if (typeof window !== "undefined") {
            window.localStorage.setItem("snappaste-theme", theme);
        }
        if (theme === "system") {
            // system 主题：设为 auto，由 CSS @media (prefers-color-scheme) 自动检测
            root.setAttribute("data-theme", "auto");
            return;
        }
        root.setAttribute("data-theme", theme);
    }

    function focusSearchInput(delayMs: number = 0) {
        setTimeout(() => {
            if (
                settingsOpen ||
                clearConfirmOpen ||
                addFavoriteOpen ||
                aboutOpen
            )
                return;
            searchBarRef?.focusInput?.();
        }, delayMs);
    }

    async function loadSettings() {
        try {
            settings = await invoke<Settings>("get_app_settings");
            applyTheme(settings.theme);
        } catch (error) {
            console.error("Failed to load settings:", error);
            toast.error(`加载设置失败：${errorMessage(error)}`);
        }
    }

    async function saveSettings(nextSettings: Settings) {
        try {
            await invoke("save_app_settings", { settings: nextSettings });
            settings = await invoke<Settings>("get_app_settings");
            applyTheme(settings.theme);
            settingsOpen = false;
            await history.load();
            focusSearchInput(0);
        } catch (error) {
            console.error("Failed to save settings:", error);
            toast.error(`保存设置失败：${errorMessage(error)}`);
        }
    }

    function openAddFavoriteDialog() {
        addFavoriteOpen = true;
        favoriteInput = "";
    }

    function closeAddFavoriteDialog() {
        if (addFavoriteSaving) return;
        addFavoriteOpen = false;
        focusSearchInput(0);
    }

    async function submitAddFavorite() {
        const text = favoriteInput.trim();
        if (!text || addFavoriteSaving) return;

        addFavoriteSaving = true;
        const saved = await history.addFavorite(text);
        if (saved) {
            addFavoriteOpen = false;
            favoriteInput = "";
            focusSearchInput(0);
        }
        addFavoriteSaving = false;
    }

    async function handleClearAll() {
        clearConfirmOpen = true;
    }

    async function confirmClearAll() {
        await history.clear();
        clearConfirmOpen = false;
        focusSearchInput(0);
    }

    function openExportFromSettings() {
        settingsOpen = false;
        focusSearchInput(0);
        void (async () => {
            try {
                await invoke("suspend_auto_hide", { ms: 10000 });
                const selected = await openDialog({
                    multiple: false,
                    directory: true,
                });
                if (!selected || Array.isArray(selected)) return;

                const result = await invoke<ExportFavoritesResult>(
                    "export_favorites_to_path",
                    {
                        path: selected,
                    },
                );
                toast.success(`已导出 ${result.count} 条收藏：${result.path}`, 6000);
            } catch (error) {
                toast.error(`导出失败：${errorMessage(error)}`);
            }
        })();
    }

    function openImportFromSettings() {
        settingsOpen = false;
        focusSearchInput(0);
        void (async () => {
            try {
                await invoke("suspend_auto_hide", { ms: 10000 });
                const selected = await openDialog({
                    multiple: false,
                    directory: false,
                    filters: [{ name: "JSON", extensions: ["json"] }],
                });
                if (!selected || Array.isArray(selected)) return;
                const [count, settingsImported] = await invoke<
                    [number, boolean]
                >("import_favorites_from_path", { path: selected });
                let msg = `导入完成，新增 ${count} 条收藏`;
                if (settingsImported) {
                    msg += "，设置已更新";
                }
                toast.success(msg);
                await history.load();
                await loadSettings();
            } catch (error) {
                toast.error(`导入失败：${errorMessage(error)}`);
            }
        })();
    }

    onMount(() => {
        let destroyed = false;
        let disposeEvents: (() => void) | undefined;

        void invoke("set_frontend_ready").catch((error) => {
            console.error("Failed to notify frontend ready:", error);
        });

        // 监听系统主题变化
        const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
        const handleThemeChange = () => {
            // 只有当前是系统主题模式时才更新
            const savedTheme = window.localStorage.getItem("snappaste-theme");
            if (savedTheme === "system" || savedTheme === "auto") {
                applyTheme("system");
            }
        };
        mediaQuery.addEventListener("change", handleThemeChange);
        void loadSettings();
        getVersion()
            .then((v) => {
                appVersion = v;
            })
            .catch((error) => {
                console.error("Failed to load app version:", error);
            });

        void registerAppEvents({
            onOpenSettings: () => {
                settingsOpen = true;
            },
            onOpenAbout: () => {
                settingsOpen = false;
                aboutOpen = true;
            },
            onMainWindowOpened: () => {
                const listEl = document.querySelector(".clipboard-list");
                if (listEl) listEl.scrollTop = 0;
                void history.resetSearch();
                focusSearchInput(16);
            },
            onHistoryChanged: () => history.scheduleRefresh(),
            onHotkeyRegisterFailed: (message) => {
                console.error("Hotkey registration failed:", message);
                hotkeyErrorMessage = message;
                hotkeyErrorOpen = true;
            },
        })
            .then((dispose) => {
                if (destroyed) dispose();
                else disposeEvents = dispose;
            })
            .catch((error) => {
                console.error("Failed to register app events:", error);
                toast.error(`初始化事件监听失败：${errorMessage(error)}`);
            });

        void history.refresh();
        focusSearchInput(16);

        return () => {
            destroyed = true;
            disposeEvents?.();
            mediaQuery.removeEventListener("change", handleThemeChange);
            history.dispose();
        };
    });

    async function handleMouseDown(e: MouseEvent) {
        // 只有左键点击且不是点击在按钮上时才触发拖动
        if (e.button === 0 && !(e.target as HTMLElement).closest("button")) {
            // 防止浏览器处理点击事件，避免导致搜索栏光标在系统接管拖拽后卡死
            e.preventDefault();
            try {
                await invoke("start_window_drag");
            } catch (error) {
                console.error("Failed to start drag:", error);
                toast.error(`无法拖动窗口：${errorMessage(error)}`);
            }
        }
    }
</script>

<main class="app">
    <header class="header" role="presentation" onmousedown={handleMouseDown}>
        <h1>{history.pageTitle}</h1>
        <div class="header-actions">
            <button
                class="refresh-btn danger"
                onclick={handleClearAll}
                aria-label={history.favoritesOnly ? "清空收藏" : "清空历史"}
            >
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
                    <path d="M10 11v6M14 11v6" />
                </svg>
            </button>
            <button
                class="refresh-btn add-favorite-btn"
                onclick={openAddFavoriteDialog}
                aria-label="添加收藏"
            >
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path d="M12 5v14M5 12h14" />
                </svg>
            </button>
            <button
                class="refresh-btn favorite-toggle"
                class:active={history.favoritesOnly}
                onclick={() => {
                    void history.toggleView().then(() => focusSearchInput());
                }}
                aria-label={history.favoritesOnly ? "切换到记录" : "切换到收藏"}
            >
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path
                        d="M12 3l2.9 5.88 6.49.95-4.7 4.58 1.11 6.47L12 17.8l-5.8 3.08 1.1-6.47-4.7-4.58 6.5-.95z"
                    />
                </svg>
            </button>
        </div>
    </header>

    <div class="search-container">
        <SearchBar
            bind:this={searchBarRef}
            bind:value={history.keyword}
            placeholder={`${history.records.length} 条记录`}
            onchange={(value) => history.search(value)}
        />
    </div>

    <div class="list-container">
        <ClipboardList
            records={history.records}
            loading={history.loading}
            oncopy={(id) => history.paste(id)}
            ondelete={(id) => history.delete(id)}
            onfavorite={(id, favorite) => history.setFavorite(id, favorite)}
            onpin={(id, pinned) => history.setPinned(id, pinned)}
            emptyTitle={history.emptyTitle}
            emptyHint={history.emptyHint}
        />
    </div>

    <SettingsModal
        open={settingsOpen}
        {settings}
        onsave={saveSettings}
        onopenimport={openImportFromSettings}
        onopenexport={openExportFromSettings}
        onclose={() => {
            settingsOpen = false;
            focusSearchInput(0);
        }}
    />

    <AppDialogs
        {addFavoriteOpen}
        bind:favoriteInput
        {addFavoriteSaving}
        {aboutOpen}
        {appVersion}
        clearOpen={clearConfirmOpen}
        favoritesOnly={history.favoritesOnly}
        {hotkeyErrorOpen}
        {hotkeyErrorMessage}
        hotkey={settings.hotkey}
        onCloseAddFavorite={closeAddFavoriteDialog}
        onSubmitFavorite={submitAddFavorite}
        onCloseAbout={() => {
            aboutOpen = false;
            focusSearchInput();
        }}
        onCancelClear={() => {
            clearConfirmOpen = false;
            focusSearchInput();
        }}
        onConfirmClear={confirmClearAll}
        onCloseHotkeyError={() => {
            hotkeyErrorOpen = false;
            focusSearchInput();
        }}
    />

    <Dialog />
    <ToastHost />
</main>

<style>
    :global(body) {
        margin: 0;
        padding: 0;
        overflow: hidden;
        background: transparent;
    }

    :global(*) {
        box-sizing: border-box;
    }

    :global(:root) {
        --bg-primary: rgba(255, 255, 255, 0.75);
        --bg-secondary: rgba(248, 249, 250, 0.8);
        --bg-hover: rgba(243, 244, 246, 0.75);
        --text-primary: #000000;
        --text-secondary: #646b77;
        --text-tertiary: #646b77;
        --border-color: rgba(209, 213, 219, 0.7);
        --accent-color: #2563eb;
        --accent-light: rgba(37, 99, 235, 0.12);
        --danger-color: #ef4444;
        --danger-light: rgba(239, 68, 68, 0.12);
        --scrollbar-track: rgba(238, 242, 247, 0.5);
        --scrollbar-thumb: rgba(198, 205, 216, 0.7);
        --scrollbar-thumb-hover: #aeb7c4;
        --glass-border: rgba(255, 255, 255, 0.8);
        --glass-shadow: rgba(0, 0, 0, 0.15);
    }

    :global([data-theme="dark"]) {
        --bg-primary: rgba(30, 30, 40, 0.65);
        --bg-secondary: rgba(40, 40, 55, 0.7);
        --bg-hover: rgba(82, 82, 95, 0.6);
        --text-primary: #f9fafb;
        --text-secondary: #9ca3af;
        --text-tertiary: #a1a7b0;
        --border-color: rgba(55, 65, 81, 0.5);
        --accent-color: #60a5fa;
        --accent-light: rgba(96, 165, 250, 0.2);
        --danger-color: #f87171;
        --danger-light: rgba(248, 113, 113, 0.2);
        --scrollbar-track: rgba(40, 40, 55, 0.3);
        --scrollbar-thumb: rgba(89, 98, 117, 0.5);
        --scrollbar-thumb-hover: #727e95;
        --glass-border: rgba(255, 255, 255, 0.1);
        --glass-shadow: rgba(0, 0, 0, 0.3);
    }

    /* 跟随系统主题 */
    @media (prefers-color-scheme: dark) {
        :global([data-theme="auto"]) {
            --bg-primary: rgba(30, 30, 40, 0.75);
            --bg-secondary: rgba(40, 40, 55, 0.7);
            --bg-hover: rgba(55, 55, 75, 0.6);
            --text-primary: #f9fafb;
            --text-secondary: #9ca3af;
            --text-tertiary: #a1a7b0;
            --border-color: rgba(55, 65, 81, 0.5);
            --accent-color: #60a5fa;
            --accent-light: rgba(96, 165, 250, 0.2);
            --danger-color: #f87171;
            --danger-light: rgba(248, 113, 113, 0.2);
            --scrollbar-track: rgba(40, 40, 55, 0.3);
            --scrollbar-thumb: rgba(89, 98, 117, 0.5);
            --scrollbar-thumb-hover: #727e95;
            --glass-border: rgba(255, 255, 255, 0.1);
            --glass-shadow: rgba(0, 0, 0, 0.3);
        }
    }

    :global(*) {
        scrollbar-width: thin;
        scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
    }

    :global(*::-webkit-scrollbar) {
        width: 10px;
        height: 10px;
    }

    :global(*::-webkit-scrollbar-track) {
        background: var(--scrollbar-track);
    }

    :global(*::-webkit-scrollbar-thumb) {
        background: var(--scrollbar-thumb);
        border-radius: 8px;
        border: 2px solid var(--scrollbar-track);
    }

    :global(*::-webkit-scrollbar-thumb:hover) {
        background: var(--scrollbar-thumb-hover);
    }

    .app {
        display: flex;
        flex-direction: column;
        height: 100vh;
        background: var(--bg-primary);
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
            "Helvetica Neue", Arial, sans-serif;
        border: 1px solid var(--border-color);
        box-shadow: 0 8px 32px var(--glass-shadow);
        overflow: hidden;
    }

    .header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 14px 18px;
        border-bottom: 1px solid var(--border-color);
        background: var(--bg-primary);
        cursor: default;
        user-select: none;
    }

    h1 {
        margin: 0;
        font-size: 15px;
        font-weight: 600;
        color: var(--text-primary);
        letter-spacing: -0.01em;
    }

    .header-actions {
        display: flex;
        gap: 8px;
    }

    .refresh-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 34px;
        height: 34px;
        padding: 0;
        border: 1px solid transparent;
        background: var(--bg-secondary);
        cursor: pointer;
        border-radius: 10px;
        transition: all 0.2s ease;
    }

    .refresh-btn:hover {
        background: var(--bg-hover);
        border-color: var(--border-color);
        transform: translateY(-1px);
    }

    .add-favorite-btn:hover {
        background: rgba(37, 99, 235, 0.14);
    }

    .add-favorite-btn:hover svg {
        color: var(--accent-color);
    }

    .refresh-btn.danger:hover {
        background: var(--danger-light);
    }

    .refresh-btn.danger:hover svg {
        color: var(--danger-color) !important;
    }

    .refresh-btn svg {
        width: 18px;
        height: 18px;
        color: var(--text-tertiary);
    }

    .search-container {
        padding: 14px 18px;
        border-bottom: 1px solid var(--border-color);
        background: var(--bg-secondary);
    }

    .list-container {
        flex: 1;
        min-height: 0;
        display: flex;
    }

    .favorite-toggle.active {
        background: rgba(245, 158, 11, 0.14);
    }

    .favorite-toggle:hover {
        background: rgba(245, 158, 11, 0.14);
    }

    .favorite-toggle:hover svg {
        color: #f59e0b;
        fill: rgba(245, 158, 11, 0.22);
    }

    .favorite-toggle.active svg {
        color: #f59e0b;
        fill: rgba(245, 158, 11, 0.22);
    }

    .refresh-btn:active {
        transform: scale(0.96);
        box-shadow: none;
    }
</style>
