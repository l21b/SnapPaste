export interface ClipboardRecord {
    id: number;
    content_type: 'text' | 'image' | 'html' | 'link';
    content: string;
    image_data?: number[] | null;
    is_favorite: boolean;
    is_pinned: boolean;
    created_at: string;
}

export interface Settings {
    hotkey: string;
    theme: 'light' | 'dark' | 'system';
    keep_days: number;
    max_records: number;
    auto_start: boolean;
    // AI settings
    ai_enabled: boolean;
    ai_hotkey: string;
    ai_api_url: string;
    ai_api_key: string;
    ai_model: string;
    ai_prompt: string;
    ai_temperature: number;
}
