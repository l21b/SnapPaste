# Glyph origin rounding fix

Source: crates.io i-slint-renderer-software 1.17.1. Original licenses retained.
Only lib.rs draw_glyph_run is changed: round the global physical glyph origin
instead of truncating it, matching the clip rectangle rounding in that function.
At a fractional position such as the centered settings panel x=10.56, the old
code drew a left-aligned glyph one pixel before its clip and cut its first column.
The fix also applies at non-integer display scales. Reassess on Slint upgrades.
