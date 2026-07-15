import { invoke } from "@tauri-apps/api/core";
import {
    listCommand,
    searchCommand,
    sortRecordsByPinnedAndTime,
} from "$lib/clipboard";
import { errorMessage } from "$lib/errors";
import { toast } from "$lib/toast.svelte";
import type { ClipboardRecord } from "$lib/types";

interface HistoryOptions {
    getLimit: () => number;
    isRefreshBlocked: () => boolean;
}

export class ClipboardHistory {
    records = $state<ClipboardRecord[]>([]);
    loading = $state(false);
    keyword = $state("");
    favoritesOnly = $state(false);

    private searchTimer: ReturnType<typeof setTimeout> | undefined;
    private refreshTimer: ReturnType<typeof setTimeout> | undefined;
    private requestSequence = 0;

    constructor(private readonly options: HistoryOptions) {}

    get pageTitle() {
        return this.favoritesOnly ? "收藏" : "历史记录";
    }

    get emptyTitle() {
        return this.favoritesOnly ? "暂无收藏" : "暂无记录";
    }

    get emptyHint() {
        return this.favoritesOnly ? "点击+来添加" : "复制内容以记录";
    }

    private async fetchRecords(): Promise<ClipboardRecord[]> {
        const keyword = this.keyword.trim();
        if (keyword) {
            return invoke<ClipboardRecord[]>(
                searchCommand(this.favoritesOnly),
                { keyword, limit: this.options.getLimit() },
            );
        }

        return invoke<ClipboardRecord[]>(listCommand(this.favoritesOnly), {
            limit: this.options.getLimit(),
            offset: 0,
        });
    }

    async load(showLoading = true) {
        const requestId = ++this.requestSequence;
        if (showLoading) this.loading = true;

        try {
            const records = await this.fetchRecords();
            if (requestId === this.requestSequence) this.records = records;
        } catch (error) {
            if (requestId === this.requestSequence) {
                console.error("Failed to load clipboard history:", error);
                toast.error(`加载记录失败：${errorMessage(error)}`);
            }
        } finally {
            if (showLoading && requestId === this.requestSequence) {
                this.loading = false;
            }
        }
    }

    async refresh() {
        if (this.options.isRefreshBlocked() || this.keyword.trim()) return;
        await this.load(false);
    }

    scheduleRefresh(delayMs = 120) {
        if (this.refreshTimer) clearTimeout(this.refreshTimer);
        this.refreshTimer = setTimeout(() => {
            this.refreshTimer = undefined;
            void this.refresh();
        }, delayMs);
    }

    search(value: string) {
        this.keyword = value;
        this.requestSequence += 1;
        if (this.searchTimer) clearTimeout(this.searchTimer);
        this.searchTimer = setTimeout(() => {
            this.searchTimer = undefined;
            void this.load();
        }, 300);
    }

    async resetSearch() {
        if (!this.keyword.trim()) return;
        if (this.searchTimer) clearTimeout(this.searchTimer);
        this.searchTimer = undefined;
        this.keyword = "";
        await this.load(false);
    }

    async toggleView() {
        this.favoritesOnly = !this.favoritesOnly;
        this.keyword = "";
        await this.load(false);
    }

    async paste(id: number) {
        if (!this.records.some((record) => record.id === id)) return;
        try {
            await invoke("paste_record_content", { id });
        } catch (error) {
            console.error("Failed to paste record:", error);
            toast.error(`粘贴失败：${errorMessage(error)}`);
        }
    }

    async delete(id: number) {
        try {
            await invoke("delete_clipboard_record", { id });
            this.records = this.records.filter((record) => record.id !== id);
        } catch (error) {
            console.error("Failed to delete record:", error);
            toast.error(`删除失败：${errorMessage(error)}`);
        }
    }

    async setFavorite(id: number, favorite: boolean) {
        const previous = this.records;
        this.records =
            this.favoritesOnly && !favorite
                ? this.records.filter((record) => record.id !== id)
                : this.records.map((record) =>
                      record.id === id
                          ? { ...record, is_favorite: favorite }
                          : record,
                  );

        try {
            await invoke("set_record_favorite_state", { id, favorite });
        } catch (error) {
            this.records = previous;
            console.error("Failed to update favorite state:", error);
            toast.error(`更新收藏失败：${errorMessage(error)}`);
        }
    }

    async setPinned(id: number, pinned: boolean) {
        const previous = this.records;
        this.records = sortRecordsByPinnedAndTime(
            this.records.map((record) =>
                record.id === id ? { ...record, is_pinned: pinned } : record,
            ),
        );

        try {
            await invoke("set_record_pinned_state", { id, pinned });
        } catch (error) {
            this.records = previous;
            console.error("Failed to update pinned state:", error);
            toast.error(`更新置顶失败：${errorMessage(error)}`);
        }
    }

    async addFavorite(content: string): Promise<boolean> {
        try {
            await invoke("add_custom_favorite_record", { content });
            await this.load();
            return true;
        } catch (error) {
            console.error("Failed to add favorite:", error);
            toast.error(`添加收藏失败：${errorMessage(error)}`);
            return false;
        }
    }

    async clear(): Promise<boolean> {
        try {
            await invoke(
                this.favoritesOnly
                    ? "clear_favorite_items"
                    : "clear_history_only",
            );
            await this.load(false);
            return true;
        } catch (error) {
            console.error("Failed to clear records:", error);
            toast.error(`清空失败：${errorMessage(error)}`);
            return false;
        }
    }

    dispose() {
        this.requestSequence += 1;
        if (this.searchTimer) clearTimeout(this.searchTimer);
        if (this.refreshTimer) clearTimeout(this.refreshTimer);
        this.searchTimer = undefined;
        this.refreshTimer = undefined;
    }
}
