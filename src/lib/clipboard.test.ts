import { describe, expect, it } from "vitest";
import {
    listCommand,
    searchCommand,
    sortRecordsByPinnedAndTime,
} from "$lib/clipboard";
import type { ClipboardRecord } from "$lib/types";

function record(
    id: number,
    createdAt: string,
    isPinned = false,
): ClipboardRecord {
    return {
        id,
        content_type: "text",
        content: `record-${id}`,
        image_data: null,
        is_favorite: false,
        is_pinned: isPinned,
        created_at: createdAt,
    };
}

describe("clipboard commands", () => {
    it("selects history and favorite command variants", () => {
        expect(listCommand(false)).toBe("get_history_records");
        expect(listCommand(true)).toBe("get_favorite_records");
        expect(searchCommand(false)).toBe("search_records");
        expect(searchCommand(true)).toBe("search_favorite_records");
    });

    it("sorts pinned records first and uses id as a stable tiebreaker", () => {
        const source = [
            record(1, "2026-01-01T00:00:00Z"),
            record(2, "2026-01-01T00:00:00Z"),
            record(3, "2025-01-01T00:00:00Z", true),
        ];

        const sorted = sortRecordsByPinnedAndTime(source);
        expect(sorted.map((item) => item.id)).toEqual([3, 2, 1]);
        expect(source.map((item) => item.id)).toEqual([1, 2, 3]);
    });
});
