export const MODIFIER_OPTIONS = [
    { value: "", label: "无修饰键" },
    { value: "Ctrl", label: "Ctrl" },
    { value: "Alt", label: "Alt" },
    { value: "Shift", label: "Shift" },
    { value: "Ctrl+Alt", label: "Ctrl + Alt" },
    { value: "Ctrl+Shift", label: "Ctrl + Shift" },
    { value: "Alt+Shift", label: "Alt + Shift" },
] as const;

function normalizeModifiers(values: string[]): string {
    return ["Ctrl", "Alt", "Shift"]
        .filter((modifier) => values.includes(modifier))
        .join("+");
}

export function normalizeKeyToken(raw: string): string {
    const token = raw.trim();
    if (!token) return "";
    if (/^[a-zA-Z0-9]$/.test(token)) return token.toUpperCase();

    const upper = token.toUpperCase();
    if (/^KEY[A-Z]$/.test(upper)) return upper.slice(3);
    if (/^DIGIT[0-9]$/.test(upper)) return upper.slice(5);
    if (/^F([1-9]|1[0-9]|2[0-4])$/.test(upper)) return upper;
    if (/^NUMPAD[0-9]$/.test(upper)) return `Numpad${upper.slice(6)}`;

    switch (upper) {
        case "ESCAPE":
        case "ESC":
            return "Esc";
        case "SPACE":
        case "SPACEBAR":
            return "Space";
        case "ARROWUP":
            return "ArrowUp";
        case "ARROWDOWN":
            return "ArrowDown";
        case "ARROWLEFT":
            return "ArrowLeft";
        case "ARROWRIGHT":
            return "ArrowRight";
        default:
            return token;
    }
}

export function parseHotkey(hotkey: string): {
    modifier: string;
    key: string;
} {
    const tokens = hotkey
        .split("+")
        .map((token) => token.trim())
        .filter(Boolean);
    if (tokens.length === 0) return { modifier: "Alt", key: "Z" };

    const key = normalizeKeyToken(tokens.at(-1) ?? "") || "Z";
    const modifiers = tokens.slice(0, -1).map((token) => {
        switch (token.toUpperCase()) {
            case "CTRL":
            case "CONTROL":
                return "Ctrl";
            case "ALT":
            case "OPTION":
                return "Alt";
            case "SHIFT":
                return "Shift";
            default:
                return "";
        }
    });
    const modifier = normalizeModifiers(modifiers);

    return {
        modifier: MODIFIER_OPTIONS.some((option) => option.value === modifier)
            ? modifier
            : "",
        key,
    };
}

export function formatHotkey(modifier: string, key: string): string {
    return modifier ? `${modifier}+${key}` : key;
}

export function keyLabel(token: string): string {
    return /^Numpad[0-9]$/.test(token)
        ? token.replace("Numpad", "Num")
        : token;
}

export function keyTokenFromEvent(event: KeyboardEvent): string {
    if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) return "";
    if (/^Key[A-Z]$/.test(event.code)) return event.code.slice(3);
    if (/^Digit[0-9]$/.test(event.code)) return event.code.slice(5);
    if (/^Numpad[0-9]$/.test(event.code)) return event.code;
    if (/^F([1-9]|1[0-9]|2[0-4])$/i.test(event.code)) {
        return event.code.toUpperCase();
    }
    if (/^[a-zA-Z0-9]$/.test(event.key)) return event.key.toUpperCase();

    const namedKeys: Record<string, string> = {
        ArrowUp: "ArrowUp",
        ArrowDown: "ArrowDown",
        ArrowLeft: "ArrowLeft",
        ArrowRight: "ArrowRight",
        Enter: "Enter",
        Tab: "Tab",
        Escape: "Esc",
        Backspace: "Backspace",
        Delete: "Delete",
        Insert: "Insert",
        Home: "Home",
        End: "End",
        PageUp: "PageUp",
        PageDown: "PageDown",
        " ": "Space",
    };
    return namedKeys[event.key] ?? normalizeKeyToken(event.key);
}
