export type ToastKind = "info" | "success" | "error";

export interface ToastMessage {
    id: number;
    kind: ToastKind;
    text: string;
}

let nextId = 1;
let messages = $state<ToastMessage[]>([]);
const timers = new Map<number, ReturnType<typeof setTimeout>>();

function remove(id: number) {
    const timer = timers.get(id);
    if (timer) clearTimeout(timer);
    timers.delete(id);
    messages = messages.filter((message) => message.id !== id);
}

function show(text: string, kind: ToastKind, durationMs?: number) {
    const duplicate = messages.find(
        (message) => message.kind === kind && message.text === text,
    );
    if (duplicate) remove(duplicate.id);

    const id = nextId++;
    const duration = durationMs ?? (kind === "error" ? 5000 : 3200);
    const nextMessages = [...messages, { id, kind, text }];
    for (const discarded of nextMessages.slice(0, -4)) {
        const timer = timers.get(discarded.id);
        if (timer) clearTimeout(timer);
        timers.delete(discarded.id);
    }
    messages = nextMessages.slice(-4);
    timers.set(id, setTimeout(() => remove(id), duration));
}

export const toast = {
    get messages() {
        return messages;
    },
    info(text: string, durationMs?: number) {
        show(text, "info", durationMs);
    },
    success(text: string, durationMs?: number) {
        show(text, "success", durationMs);
    },
    error(text: string, durationMs?: number) {
        show(text, "error", durationMs);
    },
    remove,
};
