export interface ClipboardRecord {
    id: number;
    content_type: 'text' | 'image' | 'html' | 'link';
    content: string;
    image_data?: number[] | null;
    is_favorite: boolean;
    is_pinned: boolean;
    source_app: string;
    created_at: string;
}

export interface Settings {
    hotkey_modifiers: number;
    hotkey_key: number;
    hotkey: string;
    theme: 'light' | 'dark' | 'system';
    keep_days: number;
    max_records: number;
    auto_start: boolean;
}
