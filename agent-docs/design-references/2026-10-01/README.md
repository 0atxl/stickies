# Updated UI references, October 1, 2026

Unchanged copies of the updated [UI/UX spec](stickies-uiux-spec.md) and [Settings image](settings.png) supplied from `/home/rinzler/Downloads`. The September 30 reference set remains preserved separately. No new SVG pack or Rust border reference was supplied with this update.

Use [the implementation plan](../../ui-implementation-plan.md) for the current agreed behavior and implementation status. The attachment's recipes and its claim to supersede older documents do not authorize implementation or override the user's direct clarifications. The user requested reference storage and documentation updates only, with no implementation started.

## Changes from the September 30 spec

Only sections 3.1 and 3.3 changed. The rest of the document is unchanged.

| Area | Updated text | Current status |
|---|---|---|
| Individual tools | A floating popover replaces the permanent rail that widened the card. | Implemented in the stage 1 preview. Opening tools preserves note dimensions. |
| Infinity and Pin | Infinity is retired and Pin belongs in the individual panel. | Implemented. Individual Pin means Always open; All Notes Pin controls ordering. |
| Text controls | Per-note Aa means S/M/L and Regular/Bold; font family belongs only in global Settings. | Implemented with persisted per-note size/weight. Numeric sizes remain provisional. |
| Settings content | Appearance, Edge Strip, Behavior, Shortcuts, and About are described at control level. | New planning detail and image; section list remains explicitly draft/open. |
| Settings appearance | One scrollable surface with section headers, dividers, switches, outlined selected segments, and app UI typography. | Reference received; not implemented. |

The image shows Theme, global Font, Paper pattern, Reduce motion; strip enablement, anchor, position and Reposition; launch at login and default note color; shortcut badges; app icon/version and GitHub link. Its selected theme, enabled switches, four color samples, shortcut combinations and version are examples, not saved defaults or evidence that those controls already work.

## Carry-forward conflicts

- The 40px rail and card-growth sentence remain below the new paragraph that explicitly supersedes them. Use the floating panel with direct compact rows.
- Older Pin prose asks for compositor pinning and blocked Close. Keep the user's separate ordering/Always open roles and working Close/Escape. No desktop configuration change follows from this reference.
- Unchanged font suggestions, 18px body default and 1100x620 All Notes dimensions do not replace the chosen fonts, provisional sizes or compact 720x500 default.
- The Settings image and draft show four colors. Keep the user's broader shared palette requirement; the screenshot does not cap it at four.
- The Settings image omits Minimal's System/Light/Dark controls. Preserve that capability; settle its placement in the Settings design.
- Along-edge Top/Center/Bottom labels do not cover horizontal anchors clearly. Settle horizontal labels before implementing position presets.
- Theme-following docked cards and pinned-first ordering were already settled by the user. Their old OPEN notes are stale.
- Clickable checklists already exist. Their appearance and revised item behavior remain pending; the spec's statement that interactive state is not built is stale.
- Shortcut badges are examples. Exact defaults, registration and whether they are editable remain unresolved. Do not display example combinations as active bindings.
- Settings host/width and the Reposition mode's visual treatment, save/cancel behavior and exit controls still need settling. The portrait image is not a required pixel size.

The implementation plan contains the proposed next steps and acceptance checks. Original source files in this directory stay unchanged so future updates can be compared.
