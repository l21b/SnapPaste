import { describe, expect, it } from "vitest";
import { errorMessage } from "$lib/errors";
import {
    formatHotkey,
    keyTokenFromEvent,
    normalizeKeyToken,
    parseHotkey,
} from "$lib/hotkey";
import {
    createDefaultSettings,
    getHistoryQueryLimit,
    MAX_UI_RECORDS,
} from "$lib/settings";

describe("settings utilities", () => {
    it("keeps frontend defaults aligned with the current shortcut", () => {
        const settings = createDefaultSettings();
        expect(settings.hotkey).toBe("Alt+Z");
        expect(settings.ai_enabled).toBe(false);
    });

    it("caps each UI query while allowing unlimited storage", () => {
        expect(getHistoryQueryLimit(0)).toBe(MAX_UI_RECORDS);
        expect(getHistoryQueryLimit(-1)).toBe(MAX_UI_RECORDS);
        expect(getHistoryQueryLimit(20)).toBe(20);
        expect(getHistoryQueryLimit(10_000)).toBe(MAX_UI_RECORDS);
    });
});

describe("hotkey utilities", () => {
    it("normalizes aliases and modifier order", () => {
        expect(parseHotkey("shift+control+keyv")).toEqual({
            modifier: "Ctrl+Shift",
            key: "V",
        });
        expect(parseHotkey("")).toEqual({ modifier: "Alt", key: "Z" });
        expect(formatHotkey("Alt", "Z")).toBe("Alt+Z");
    });

    it("normalizes named and numpad keys", () => {
        expect(normalizeKeyToken("escape")).toBe("Esc");
        expect(normalizeKeyToken("numpad7")).toBe("Numpad7");
        expect(
            keyTokenFromEvent({ key: "a", code: "KeyA" } as KeyboardEvent),
        ).toBe("A");
    });
});

describe("error formatting", () => {
    it("produces readable messages for common thrown values", () => {
        expect(errorMessage(new Error("failed"))).toBe("failed");
        expect(errorMessage("failed")).toBe("failed");
        expect(errorMessage({ code: 5 })).toBe('{"code":5}');
        expect(errorMessage(undefined)).toBe("undefined");
    });
});
