# Current milestone

The owner accepts the current UI/UX. Design is complete; the active milestone is reliability, Wayland verification and release finishing for a simple sticky-notes app. Verification does not reopen settled design choices.

[SPEC.md](SPEC.md) describes current behavior. [implementation.md](implementation.md) owns remaining roadmap work. The [detailed verification record](manual-testing.md) contains results, fixtures, evidence and the remaining checklist. Further test execution requires separate authorization under [AGENTS.md](../AGENTS.md).

## Committed work

- Mechanical cleanup and documentation were committed as `8750f23`.
- Historical review cleanup and the four finishing tweaks were committed as `7cfdc23`: empty-app welcome/conditional filters, persistent random new-note colors, repeated emoji selection and native Quick Search outside-click dismissal. Hover-to-reveal and the accepted first-click dismissal tradeoff remain unchanged.

## Verification status

The [October 16 baseline](manual-testing.md#october-16-verification-baseline) began clean at the expected `7cfdc23`. Formatting, locked compilation/builds, Clippy, static checks and 51 ordinary tests passed. Focused checks covered saving/reload, failed-save guards, drafts, shared history, formatting/Unicode/lists, representative checklist/emoji input, archive/Delete Undo, timeout/reduced motion and catchable shutdown. Ordinary fixtures used 12–14 notes, with smaller boundary fixtures and an existing 20-note migration fixture.

The baseline found Search placement/input defects and a desktop-preference integration test failure. The [focused Search fix](manual-testing.md#october-16-quick-search-follow-up) resolved both Search defects. Its affected automated checks and native CLI/D-Bus/strip opening, typing, navigation, creation, reopening and dismissal checks passed on all four edges in isolated Sway 1.12, one 1280 × 800 scale-1 output. Nearby focus, Escape and hover regressions passed. No full suite was repeated.

The owner [explicitly confirmed all requested normal-desktop Search checks passed](manual-testing.md#october-16-owner-desktop-confirmation), including four-edge placement, immediate typing from the strip/CLI and fresh startup, Up/Down/Enter, query retention/reset, reopening, Escape/outside-click dismissal and keyboard focus returning to another app. These confirmations resolve the outstanding physical-keyboard Search checks only.

Compilation/builds still report **24 known GTK deprecation warnings**. Clippy explicitly allowed deprecated APIs and found no additional warnings. Deprecated API replacement is outside this pass.

## Remaining work and limits

- The inherited-animation-preference test remains an environment failure: headless Sway/private-bus GSettings did not supply the desktop-settings bridge. App-controlled reduced motion and persistence passed separately.
- Accessibility, fractional/high scaling, fullscreen policy, installed login/session-end, five-tab UI allocation beyond storage boundaries, complete emoji keyboard navigation/frame-level flicker, every checklist caret/wrap case, resource/lifetime observations and clean-runner CI remain unverified. See the detailed record for limits.
- Isolated Hyprland could not start without `/dev/dri`. The owner's confirmation does not identify compositor/version or display setup. These results establish no broad Wayland compatibility claim; additional compositors need actual app verification.
- Shared-monitor selection and Layer Shell v4+ startup checks are implemented. Earlier rebuilt-binary checks failed resolution placement and no-match Search. The owner explicitly confirmed resolution recovery passes, and a temporary full-redraw launch improved Search. Resolution and full-redraw captures were truncated; the complete Search capture showed a size transition followed by stable geometry. Rendering root cause remains unestablished. The owner accepted fixed-size Search with a permanent 128-pixel results area and explicitly confirmed no-match jitter was gone. Temporary diagnostics have been removed. Cleanup has static verification only; no new app or test sessions ran. Focus previously passed. Multi-monitor behavior and protocol rejection remain unverified, so monitor recovery is not complete. Desktop settings dismisses Search normally, so resolution changes with Search open were not exercised. Packaging/application icon remain separate roadmap work. All Notes' complete-body processing for short excerpts remains unchanged.

Keep ordinary verification near 10–30 notes with minimal boundary fixtures. Preserve saved data, formatting/history and compatibility. Automatic cursor-following, extra package formats, native ports and scripting/MCP remain deferred; Drive remains outside the first release. Extra Settings controls and manual drag ordering stay removed.
