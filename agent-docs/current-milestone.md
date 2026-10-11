# Current milestone

[SPEC.md](SPEC.md) is the current product truth derived from the code. [HISTORY.md](HISTORY.md) preserves the former milestone log unchanged. The active milestone remains final UI work and phase 5A/5B verification; [implementation.md](implementation.md) records later roadmap work.

## Open items

- The October 11 edge-strip polish uses 6 px tab gaps, quieter tab/capsule shadows, a single selected-tab accent and contiguous utility cells with centered dividers. Title size and hover behavior are unchanged. The normal offline build passed with existing GTK deprecation warnings. Visual approval and the reported softness remain unresolved. No tests were run at the user's request.
- Complete final manual review of recent edge chrome/motion, positioning, Search/arrow behavior, blank-note cleanup and All Notes opening without a preview. Use [manual-testing.md](manual-testing.md); older reference drafts must be checked against SPEC.
- Confirm docked timeout choices, pointer activity, Always open, persistent pop-outs, popover-first Escape and the running-instance actions in the final desktop UI.
- Settle the remaining Settings scope: true strip enablement, launch-at-login control, default new-note color and About. Obtain the checklist redesign specification before a separate implementation pass.
- The October 10 UI corrections passed 34 non-display tests and all 12 native GTK tests in a disposable Wayland session. Release checks confirmed toolbox placement, Search scrolling/Unicode activation and empty All Notes layouts. User visual approval remains pending. The icon-mask cache leaves a large-content startup/memory failure: 1,284 notes with roughly 20 KB bodies took 54.2 seconds to map All Notes and used 379.4 MiB PSS in the isolated 125% display. A bounded list excerpt is proposed, awaiting approval. See [the verification record](../plans/README.md). Recheck the old menu-retention finding against current weak callbacks before declaring it resolved; see [code-audit.md](code-audit.md).
- Complete native CI dependency coverage (libadwaita, Fontconfig and resource compilation) and verify on a clean runner. Include all bundled-font notices in distribution.
- Verify installed launch after an actual login; generating the autostart entry alone is insufficient.
- Finish fullscreen/DND, focus handoff, window movement/floating and scaling checks on the reference compositor. Portable installation must not depend on the development machine's floating rules. Wider compositor validation remains later roadmap work.
- Investigate the graphics invalid-rectangle warning if it recurs; it has not been declared fixed.
- Remeasure the final UI, including the two screen-sized layer surfaces: at most 200 MiB PSS, ordinary interaction below 2% CPU, normal animation below 3%, exceptional one-second spikes at most 5%, idle returning to 0%. Use interval measurements with labeled activity; earlier baselines are not final acceptance.
