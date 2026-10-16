# Historical cleanup completed

The owner approved all 28 original historical deletion candidates and the images/videos associated with the nine obsolete documents and live-input scripts. All approved files have been removed. No deletion candidates remain pending from this review. The owner declined backups; no backup archive is retained.

Current behavior, requirements, useful failure details and historical performance measurements are consolidated in the tracked SPEC and finishing roadmap. Runtime source, application assets, migrations, saved notes, supplied reference images/specifications and SVG packs remain unchanged. Reference index links and retained galleries were updated for the removals.

## Removed original candidates

The 18 tracked files matched their Git contents before removal and remain recoverable at `a2105f2`. Nine ignored/untracked documents/scripts had no exact Git copy. The ignored duplicate image matched the retained local image byte for byte.

| Original row | Removed file | Recovery |
|---|---|---|
| 1 | `agent-docs/ui-implementation-plan.md` | Exact copy in Git at `a2105f2` |
| 2 | `agent-docs/stickies-redo-plan.md` | No Git copy; no backup retained |
| 3 | `agent-docs/audit-2026-10-08.md` | No Git copy; no backup retained |
| 4 | `agent-docs/HISTORY.md` | No Git copy; no backup retained |
| 5 | `plans/README.md` | Exact copy in Git at `a2105f2` |
| 6 | `agent-docs/design-previews/2026-10-01-tools/README.md` | Exact copy in Git at `a2105f2` |
| 7 | `agent-docs/design-previews/2026-10-01-tools/minimal-dark-tools.png` | Exact copy in Git at `a2105f2` |
| 8 | `agent-docs/design-previews/2026-10-01-tools/minimal-light-tools.png` | Exact copy in Git at `a2105f2` |
| 9 | `agent-docs/design-previews/2026-10-01-tools/paper-18.png` | Exact copy in Git at `a2105f2` |
| 10 | `agent-docs/design-previews/2026-10-01-tools/paper-21.png` | Exact copy in Git at `a2105f2` |
| 11 | `agent-docs/design-previews/2026-10-01-tools/paper-24.png` | Exact copy in Git at `a2105f2` |
| 12 | `agent-docs/design-previews/2026-10-01-tools/paper-bold.png` | Exact copy in Git at `a2105f2` |
| 13 | `agent-docs/design-previews/2026-10-01-tools/paper-main-18.png` | Exact copy in Git at `a2105f2` |
| 14 | `agent-docs/design-previews/2026-10-01-tools/paper-main-21.png` | Exact copy in Git at `a2105f2` |
| 15 | `agent-docs/design-previews/2026-10-01-tools/paper-main-24.png` | Exact copy in Git at `a2105f2` |
| 16 | `agent-docs/design-previews/2026-10-01-tools/paper-main-bold.png` | Exact copy in Git at `a2105f2` |
| 17 | `agent-docs/design-previews/2026-10-01-tools/paper-pinned-tools.png` | Exact copy in Git at `a2105f2` |
| 18 | `agent-docs/design-previews/2026-10-01-tools/paper-tools.png` | Exact copy in Git at `a2105f2` |
| 19 | `agent-docs/design-previews/2026-10-01-tools/playful-tools.png` | Exact copy in Git at `a2105f2` |
| 20 | `agent-docs/design-previews/2026-10-01-tools/text-size-preview.png` | Exact copy in Git at `a2105f2` |
| 21 | `agent-docs/design-previews/2026-10-01-tools/tools-overview.png` | Exact copy in Git at `a2105f2` |
| 22 | `agent-docs/design-previews/2026-10-01-paper-followup/tools.png` | Identical local image retained at `2026-10-01-paper/tools.png` |
| 23 | `agent-docs/design-previews/2026-10-01-edge-strip/correction-2026-10-03/launch-review.py` | No Git copy; no backup retained |
| 24 | `agent-docs/design-previews/2026-10-07-lightweight-edge/native-review.py` | No Git copy; no backup retained |
| 25 | `agent-docs/design-previews/2026-10-07-lightweight-edge/native-theme-check.py` | No Git copy; no backup retained |
| 26 | `agent-docs/design-previews/2026-10-09-edge-hover/record.py` | No Git copy; no backup retained |
| 27 | `agent-docs/design-previews/2026-10-09-edge-motion/record.py` | No Git copy; no backup retained |
| 28 | `agent-docs/design-previews/2026-10-09-edge-motion-fixes/record.py` | No Git copy; no backup retained |

## Associated gallery media removed

The following five gallery trees contained 2,848 images and 38 videos, totaling 359,175,120 bytes, removed with owner approval. The original candidate list above adds 16 PNGs: 15 obsolete tools previews and one duplicate image.

| Gallery tree | Images | Videos |
|---|---:|---:|
| `agent-docs/design-previews/2026-10-01-edge-strip/correction-2026-10-03` | 180 | 12 |
| `agent-docs/design-previews/2026-10-07-lightweight-edge` | 259 | 5 |
| `agent-docs/design-previews/2026-10-09-edge-hover` | 311 | 7 |
| `agent-docs/design-previews/2026-10-09-edge-motion` | 1380 | 4 |
| `agent-docs/design-previews/2026-10-09-edge-motion-fixes` | 718 | 10 |

Historical logs, event/analysis metadata and offline extraction source remain. Retained galleries identify their retired media, and the October 1 paper follow-up uses the identical retained tools image. Other gallery media outside the approved groups remain in place.

## Verification and boundary

Static file-integrity and reference checks cover the approved paths, retained duplicate, current tracked documentation links and protected runtime/reference files. No tests, benchmarks or app sessions were run for historical removal. These changes remain uncommitted; further commits and pushes require explicit owner instruction. The eight unused runtime icons were approved and removed in the earlier mechanical cleanup commit `8750f23`.
