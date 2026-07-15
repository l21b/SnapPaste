import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface AppEventHandlers {
    onOpenSettings: () => void;
    onOpenAbout: () => void;
    onMainWindowOpened: () => void;
    onHistoryChanged: () => void;
    onHotkeyRegisterFailed: (message: string) => void;
}

export async function registerAppEvents(
    handlers: AppEventHandlers,
): Promise<UnlistenFn> {
    const disposers: UnlistenFn[] = [];

    try {
        disposers.push(await listen("open-settings", handlers.onOpenSettings));
        disposers.push(await listen("open-about", handlers.onOpenAbout));
        disposers.push(
            await listen("main-window-opened", handlers.onMainWindowOpened),
        );
        disposers.push(
            await listen("history-changed", handlers.onHistoryChanged),
        );
        disposers.push(
            await listen<string>("hotkey-register-failed", (event) => {
                handlers.onHotkeyRegisterFailed(event.payload);
            }),
        );
    } catch (error) {
        for (const dispose of disposers) dispose();
        throw error;
    }

    return () => {
        for (const dispose of disposers) dispose();
    };
}
