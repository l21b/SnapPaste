# Windows full-frame layered presentation

Source: crates.io softbuffer 0.4.8; original licenses retained.
Only src/backends/win32.rs changes. Layered windows (WS_EX_LAYERED) use
UpdateLayeredWindow with AC_SRC_ALPHA to submit the entire premultiplied DIB.
Position is preserved. The surface retains its initial layered mode and restores
WS_EX_LAYERED if a window library rewrites the style. This is the production
SnapPaste path; presentation errors propagate to the caller.

Regular windows retain BitBlt, acquiring a fresh window DC per presentation and
releasing it on the same thread through RAII, including errors. The DIB and memory
DC remain cached. BitBlt failure propagates instead of validating an unpainted window.

Reassess with upstream on dependency upgrades. The application-level native
regression is tests/windows_present.rs (run explicitly on an unlocked desktop).
