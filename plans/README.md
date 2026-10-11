# UI fix checklist

Implemented after user approval on October 10, 2026. The user subsequently authorized tests. Functional checks passed; the large-content performance check failed. User visual approval remains pending. Baseline references below describe `c955e0d` plus the pre-existing uncommitted work.

Stay within final UI work and phase 5A/5B. Use the existing Rust/GTK4/libadwaita code, native tools and disposable data. Keep the approved appearance, geometry and motion values: reveal 160 ms, collapse 110 ms, panel 120 ms, hover 120 ms, stagger 20 ms, hover scale 1.06 and existing easing. Settings expansion and checklist redesign remain outside this work.

## Fixes

Findings 1 through 5 and 8 were reproduced in the native app before implementation. Findings 6 and 7 were code-confirmed. The results below distinguish functional assertions, visual checks and remaining acceptance work.

| # | Problem and priority | Smallest change | Focused check |
|---|---|---|---|
| 1 | All Notes startup stalls, P1 | In `src/ui_assets.rs:86`, cache the existing decoded 36 x 36 A8 masks by resource path on the GTK thread. Keep draw-time CSS tinting. | Compare icons and states across themes; remeasure startup, CPU and PSS with small and 1,284-note collections, using short and long bodies. |
| 2 | Search keyboard selection leaves the viewport, P2 | In `src/edge.rs:1196`, retain the existing scroller and clamp its adjustment to the selected row's bounds after selection or refresh. Keep entry focus. | Move through 20+ results in both directions, edit the query and press Enter. Selection must stay visible and activate the expected note. |
| 3 | Unicode case variants miss matches, P2 | In `src/storage.rs:243`, use GLib case folding through one SQLite scalar function and fold the query once. Enable the existing `rusqlite` functions feature. | Check title/body matches for `Ólafur`/`ólafur`, `Straße`/`STRASSE` and Greek sigma in both Search views; retain literal matching, collection filters and ordering. |
| 4 | Rejected Appearance changes stay selected, P2 | In `src/settings.rs:177`, retain the current control handles and synchronize them from accepted configuration after updates and refresh. Reuse the signal guard and remove unconditional requested-state writes. | Reject a disposable config write, then retry successfully. Theme, mode, font/reset, pattern and Reduce motion must agree with live configuration without triggering another save. |
| 5 | Search blinks when reopened during its fade, P2 | In `src/edge.rs:567`, retarget the same outgoing Search panel from its painted opacity using the existing motion helper. Keep the arrow synchronized. | Record rapid close/reopen and inspect consecutive frames. Check note switches, Escape and reduced motion for stale callbacks or misplaced input bounds. |
| 6 | Settings overwrites a live animation preference, P2 | In `src/main.rs:514`, remove the startup snapshot. Force GTK animations off for app Reduce motion; otherwise call `reset_property("gtk-enable-animations")`. Apply at startup and after accepted preference changes. | Use an inherited GTK setting fixture. Check system-disabled startup, live changes, unrelated settings writes, rejected writes and disabling motion mid-animation. |
| 7 | Numbered toggle adds `1. ` before `2. `, P2 | In `src/editor.rs:636`, make the existing numbered branch remove any leading `u32` prefix followed by `. `, matching Enter's parser. | Exercise the real helper and buttons in All Notes, docked and floating notes. Check continued items, Undo, Enter and existing bullet/checklist controls. |
| 8 | Toolbox obscures the title/body, P2 | In `src/editor.rs`, anchor the existing popover to the whole sheet in its header button's coordinates. Update on opening, resize and edge configuration; let GTK flip sides when needed. | Open tools on all four edges and in moved/resized floating notes. Title, body and Close should remain unobstructed; keyboard access and Escape should still work. |

The icon profile identifies repeated decoding as a contributor. If the cache leaves a startup stall, measure the remaining cost and present a separate proposed fix. Unicode folding adds case expansions, but preserves accents and does not add normalization or change sorting.

## Verification status

The following passed on the final source:

- `cargo test --locked --offline -- --test-threads=1`: 34 passed, with 12 display tests ignored by this command and run separately.
- All 12 ignored tests passed individually in an isolated Hyprland Wayland session with private buses and disposable XDG directories. Coverage includes four themes/four edges, default/minimum sizes, hover/reveal/collapse, Search selection and interrupted fades, focus, reduced/system motion, popover-first Escape, rejected writes, shared sessions and blank-note cleanup. The first combined run passed 11; its collapse test encountered the virtual cursor over the strip. The motion test passed after parking it away, using both Cairo and the default renderer. GTK widget opacity assertions allow its 8-bit precision.
- Release SIGTERM/SIGHUP/SIGINT probes preserved pending title/body and exited successfully before autosave.
- Formatting, diff whitespace, `cargo clippy --locked --offline --all-targets -- -D warnings -A deprecated`, and the locked offline release build. The release build retains 28 existing GTK deprecation warnings.

Tests caught and corrected Search's post-layout scroll reset, an extra update after a rejected Reduce motion switch, and toolbox button direction/opening order. The storage fixtures now open through the production connection setup so the Unicode function is registered.

Visually checked release screenshots confirm the toolbox beside Paper Left/Right, Minimal Light Right, Minimal Dark Top and Playful Bottom notes, visible keyboard-selected Search results with entry focus, `ólafur` matching/activating `Ólafur`, and All Notes empty/default/minimum layouts. A native mouse resize to the individual minimum kept the open toolbox beside the resized note. The native GTK assertions also check that docked and separate toolbox contents do not intersect the body. These checks do not replace user visual approval, actual login/session-end delivery, screen-reader, multi-monitor, 200% scale or other-compositor acceptance.

Logs and disposable fixtures are preserved at `/tmp/stickies-ui-verification.DAyzps`, `/tmp/stickies-ui-audit-9nosqxq3`, `/tmp/stickies-ui-audit-otytg31j` and `/tmp/stickies-ui-audit-s2nyynjo`. The original failed-run logs are retained. The timing fixture measures process launch to first mapped All Notes window, including GTK/fonts/private-bus startup, on a 1920×1080 isolated output at 125%; it is not a before/after benchmark on the user's normal desktop.

| Collection | Largest body | First mapped All Notes | PSS |
|---|---:|---:|---:|
| 20 notes | 382 bytes | 1.33 s | 143.1 MiB |
| 1,284 notes | 382 bytes | 3.18 s | 178.2 MiB |
| 1,284 notes | 20,582 bytes | 54.18 s | 379.4 MiB |

The long-content case exceeds the 200 MiB budget. Its final four one-second idle samples were 0% CPU, after earlier settling activity. Ordinary-interaction/animation CPU budgets remain unaccepted.

Remaining P1 finding: `src/manager.rs:1141` passes the entire flattened body to every list-row label. The smallest proposed follow-up is to bound the Unicode-safe list excerpt before GTK lays it out, preserving full stored/editor text and existing row styling. Verify with the same short/long fixtures, title/body search, Unicode excerpts and the memory/startup measurements. This changes the manager's responsibility beyond the approved icon-cache correction and awaits approval under AGENTS.md. No follow-up source change has been made. Commit and push were not authorized.

Scope excludes a browser prototype, demo toggle, new UI framework, pagination, schema migration and redesign. A material expansion requires a revised proposal and approval.
