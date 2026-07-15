import type { ClipboardRecord } from "$lib/types";

export function listCommand(
    favoritesOnly: boolean,
): "get_history_records" | "get_favorite_records" {
    return favoritesOnly ? "get_favorite_records" : "get_history_records";
}

export function searchCommand(
    favoritesOnly: boolean,
): "search_records" | "search_favorite_records" {
    return favoritesOnly ? "search_favorite_records" : "search_records";
}

export function sortRecordsByPinnedAndTime(
    items: ClipboardRecord[],
): ClipboardRecord[] {
    return [...items].sort((a, b) => {
        const pinDifference = Number(b.is_pinned) - Number(a.is_pinned);
        if (pinDifference !== 0) return pinDifference;
        const timeDifference =
            new Date(b.created_at).getTime() -
            new Date(a.created_at).getTime();
        return timeDifference || b.id - a.id;
    });
}
