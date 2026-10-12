# Current milestone

[SPEC.md](SPEC.md) is the current product truth derived from the code. [HISTORY.md](HISTORY.md) preserves the former milestone log unchanged. The active milestone remains final UI work and phase 5A/5B verification; [implementation.md](implementation.md) records later roadmap work.

## October 12 removal-only controls pass

Prompt 1 of [the redo plan](stickies-redo-plan.md) removes both body editing bars, Find in note, inline formatting access, the individual nested More menu, the five retired Settings choices and edge-tab tooltips. Stacked tabs and the bar marker are the only edge presentation. Old white-paper/tab-layout/marker-style keys are removed from valid configs on load. Fullscreen/DND keys remain file-only and are documented in README.

Existing formatting storage and per-note styles remain. Individual Bullet list, Numbered list, Pattern override, Copy text and docked Open away from edge are temporarily unreachable until Prompt 2. The first tools popover and All Notes actions stay unchanged. Footer/selection-bubble rebuilding and hover peek cards await separate prompts and approval. No new UI is added.

Verification passed: 35 non-display tests, all 15 native GTK tests in an isolated Sway 125% Wayland output, three affected edge rechecks, and all three release shutdown probes. Formatting, strict Clippy with existing GTK deprecations allowed, the locked offline release build and diff checks passed. The release build retains 28 existing deprecation warnings. The requested four-theme captures and the control/storage/test report are in `/tmp/stickies-removal-check/review.md`. User review is required before Prompt 2.

The daily-use record below describes the preceding pass; this removal pass supersedes its controls and edge-style choices.

## October 12 daily-use feedback

Implemented with owner approval: All Notes tools reserve space beside or below the sheet; no new click-away closing behavior; Kalam Regular/Bold; theme-ink checklist completion and read-only archived checkboxes; optional Stacked tabs and Bar marker with existing defaults preserved; exposed working behavior settings; compositor shortcuts moved to README; shorter hover labels; body undo/redo, Find in note and selection formatting stored separately in SQLite schema 4. All Notes now requests a 680 × 520 px minimum so compact tools remain visible. Its default size is unchanged.

Verification: 34 non-display tests and all 15 native GTK tests passed, with the two settings tests rerun after correcting a rejected-switch resubmission and updating the minimum-size assertion. Strict Clippy with existing deprecations allowed, formatting, diff checks and the locked offline release build passed. The release build retains 28 existing GTK deprecation warnings. Screenshots and logs are under `/tmp/stickies-feedback-check`; the compact/wide All Notes review is in `case-2vdrszpa/images`. User visual approval remains pending.

The note-only Hyprland border rule was added to the owner's user Lua config, with a timestamped backup. Reload and config-error checks passed. Actual login launch, final performance budgets and broader compositor acceptance remain open.

## Open items

- The October 11 edge-strip polish uses 6 px tab gaps, quieter tab/capsule shadows, a single selected-tab accent and contiguous utility cells with centered dividers. Title size and hover behavior are unchanged. The normal offline build passed with existing GTK deprecation warnings. Visual approval and the reported softness remain unresolved. No tests were run at the user's request.
- Complete final manual review of recent edge chrome/motion, positioning, Search/arrow behavior, blank-note cleanup and All Notes opening without a preview. Use [manual-testing.md](manual-testing.md); older reference drafts must be checked against SPEC.
- Confirm docked timeout choices, pointer activity, Always open, persistent pop-outs, popover-first Escape and the running-instance actions in the final desktop UI.
- Settle the remaining Settings scope: true strip enablement, default new-note color and About. Shape and visible-note-count UI also remain unimplemented; `visible_notes` is still fixed to four at runtime.
- The October 10 UI corrections passed 34 non-display tests and all 12 native GTK tests in a disposable Wayland session. Release checks confirmed toolbox placement, Search scrolling/Unicode activation and empty All Notes layouts. User visual approval remains pending. The icon-mask cache leaves a large-content startup/memory failure: 1,284 notes with roughly 20 KB bodies took 54.2 seconds to map All Notes and used 379.4 MiB PSS in the isolated 125% display. A bounded list excerpt is proposed, awaiting approval. See [the verification record](../plans/README.md). Recheck the old menu-retention finding against current weak callbacks before declaring it resolved; see [code-audit.md](code-audit.md).
- Complete native CI dependency coverage (libadwaita, Fontconfig and resource compilation) and verify on a clean runner. Include all bundled-font notices in distribution.
- Verify installed launch after an actual login; generating the autostart entry alone is insufficient.
- Finish fullscreen/DND, focus handoff, window movement/floating and scaling checks on the reference compositor. Portable installation must not depend on the development machine's floating rules. Wider compositor validation remains later roadmap work.
- Investigate the graphics invalid-rectangle warning if it recurs; it has not been declared fixed.
- Remeasure the final UI, including the two screen-sized layer surfaces: at most 200 MiB PSS, ordinary interaction below 2% CPU, normal animation below 3%, exceptional one-second spikes at most 5%, idle returning to 0%. Use interval measurements with labeled activity; earlier baselines are not final acceptance.
