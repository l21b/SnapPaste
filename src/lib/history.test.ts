import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ClipboardHistory } from "$lib/history.svelte";
import { toast } from "$lib/toast.svelte";
import type { ClipboardRecord } from "$lib/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

function record(id: number, favorite = false): ClipboardRecord {
    return {
        id,
        content_type: "text",
        content: `record-${id}`,
        image_data: null,
        is_favorite: favorite,
        is_pinned: false,
        created_at: "2026-01-01T00:00:00Z",
    };
}

function deferred<T>() {
    let resolve!: (value: T) => void;
    const promise = new Promise<T>((complete) => {
        resolve = complete;
    });
    return { promise, resolve };
}

function createHistory(blocked = false) {
    return new ClipboardHistory({
        getLimit: () => 50,
        isRefreshBlocked: () => blocked,
    });
}

describe("ClipboardHistory", () => {
    beforeEach(() => {
        invokeMock.mockReset();
        vi.spyOn(toast, "error").mockImplementation(() => {});
    });

    afterEach(() => {
        vi.useRealTimers();
        vi.restoreAllMocks();
    });

    it("keeps the newest response when requests finish out of order", async () => {
        const first = deferred<ClipboardRecord[]>();
        const second = deferred<ClipboardRecord[]>();
        invokeMock
            .mockReturnValueOnce(first.promise)
            .mockReturnValueOnce(second.promise);
        const history = createHistory();

        const firstLoad = history.load(false);
        history.keyword = "new";
        const secondLoad = history.load(false);
        second.resolve([record(2)]);
        await secondLoad;
        first.resolve([record(1)]);
        await firstLoad;

        expect(history.records.map((item) => item.id)).toEqual([2]);
    });

    it("debounces search and sends only the latest keyword", async () => {
        vi.useFakeTimers();
        invokeMock.mockResolvedValue([]);
        const history = createHistory();

        history.search("first");
        history.search("second");
        await vi.advanceTimersByTimeAsync(300);

        expect(invokeMock).toHaveBeenCalledTimes(1);
        expect(invokeMock).toHaveBeenCalledWith("search_records", {
            keyword: "second",
            limit: 50,
        });
    });

    it("rolls back optimistic favorite updates when IPC fails", async () => {
        invokeMock.mockRejectedValue(new Error("denied"));
        const history = createHistory();
        history.records = [record(1)];

        await history.setFavorite(1, true);

        expect(history.records[0].is_favorite).toBe(false);
        expect(toast.error).toHaveBeenCalledWith("更新收藏失败：denied");
    });

    it("rolls back optimistic pin updates when IPC fails", async () => {
        invokeMock.mockRejectedValue(new Error("denied"));
        const history = createHistory();
        history.records = [record(1)];

        await history.setPinned(1, true);

        expect(history.records[0].is_pinned).toBe(false);
        expect(toast.error).toHaveBeenCalledWith("更新置顶失败：denied");
    });

    it("skips blocked refreshes and cancels scheduled work on dispose", async () => {
        const blockedHistory = createHistory(true);
        await blockedHistory.refresh();
        expect(invokeMock).not.toHaveBeenCalled();

        vi.useFakeTimers();
        const history = createHistory();
        history.scheduleRefresh(100);
        history.dispose();
        await vi.advanceTimersByTimeAsync(100);
        expect(invokeMock).not.toHaveBeenCalled();
    });
});
