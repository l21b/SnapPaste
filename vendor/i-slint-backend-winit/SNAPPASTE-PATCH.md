# SnapPaste Windows software-renderer patch

Source: crates.io i-slint-backend-winit 1.17.1. Original source and LICENSES retained.
renderer/sw.rs: Windows draws use NewBuffer rather than trusting
softbuffer buffer age. This renders and presents the full client area on requested
frames, including after hide/show, movement, and hover/caret updates. Other platforms
keep upstream partial rendering. No periodic refresh loop is added.

Tradeoff: more pixels per frame for the small clipboard window, in exchange for
correct transparent-window contents. Reassess against upstream when updating Slint.

Windows software windows disable winit DWM glass at creation and enable
WS_EX_LAYERED before surface creation. The patched softbuffer submits full
premultiplied frames with UpdateLayeredWindow; no visibility-clipped GDI blit.

winitwindowadapter.rs: Windows visibility changes explicitly request a frame after
mapping, so an initial hidden pre-render is never the only submit.
