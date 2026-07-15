import { listen } from "@tauri-apps/api/event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { registerAppEvents } from "$lib/tauri-events";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const listenMock = vi.mocked(listen);
const handlers = {
    onOpenSettings: vi.fn(),
    onOpenAbout: vi.fn(),
    onMainWindowOpened: vi.fn(),
    onHistoryChanged: vi.fn(),
    onHotkeyRegisterFailed: vi.fn(),
};

describe("registerAppEvents", () => {
    beforeEach(() => {
        listenMock.mockReset();
        for (const handler of Object.values(handlers)) handler.mockClear();
    });

    it("returns one disposer that unregisters every listener", async () => {
        const disposers = Array.from({ length: 5 }, () => vi.fn());
        for (const disposer of disposers) {
            listenMock.mockResolvedValueOnce(disposer);
        }

        const dispose = await registerAppEvents(handlers);
        dispose();

        expect(listenMock).toHaveBeenCalledTimes(5);
        for (const listenerDispose of disposers) {
            expect(listenerDispose).toHaveBeenCalledOnce();
        }
    });

    it("cleans up earlier listeners when later registration fails", async () => {
        const first = vi.fn();
        const second = vi.fn();
        listenMock
            .mockResolvedValueOnce(first)
            .mockResolvedValueOnce(second)
            .mockRejectedValueOnce(new Error("registration failed"));

        await expect(registerAppEvents(handlers)).rejects.toThrow(
            "registration failed",
        );
        expect(first).toHaveBeenCalledOnce();
        expect(second).toHaveBeenCalledOnce();
    });
});
