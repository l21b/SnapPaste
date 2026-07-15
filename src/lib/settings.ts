import type { Settings } from "$lib/types";

export const MAX_UI_RECORDS = 500;

export function createDefaultSettings(): Settings {
    return {
        hotkey: "Alt+Z",
        theme: "system",
        keep_days: 1,
        max_records: 500,
        auto_start: false,
        ai_enabled: false,
        ai_hotkey: "Ctrl+Shift+A",
        ai_api_url: "",
        ai_api_key: "",
        ai_model: "",
        ai_prompt:
            "你是拼音纠错专家。修正输入中的同音/简拼错误，禁止润色，严禁解释，直接输出修正后全文。",
        ai_temperature: 0.3,
    };
}

export function getHistoryQueryLimit(maxRecords: number): number {
    return maxRecords > 0
        ? Math.min(Math.trunc(maxRecords), MAX_UI_RECORDS)
        : MAX_UI_RECORDS;
}
