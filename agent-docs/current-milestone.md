# Current milestone

The owner accepts the current UI/UX and design choices. UI design is complete. The active work is reliability, Wayland compatibility and finishing a simple sticky-notes app. Remaining behavioral verification does not reopen design decisions.

The owner subsequently approved four finishing tweaks on October 16: the empty-app welcome and conditional collection filters, random persistent colors for new notes, continued emoji selection, and Quick Search outside-click dismissal. Hover-to-reveal remains unchanged. The owner accepted native popup behavior, where the dismissing outside click may require a second click to activate another application. Implementation and formatting/compile/static checks are authorized; all tests, benchmarks and app sessions are reserved for a new session. These changes do not authorize commits or pushes.

[SPEC.md](SPEC.md) describes the implementation. [implementation.md](implementation.md) classifies remaining work, dependencies and completion conditions, including [focused follow-ups](implementation.md#focused-follow-ups). [manual-testing.md](manual-testing.md) describes future verification; it is not authorization to run it. These current documents are tracked in Git.

## Current scope

- Preserve notes, separate formatting metadata, shared title/body history, lists, shortcuts, configuration compatibility and database upgrades.
- Finish saving, focus, docked-only timeout, startup/shutdown, accessibility and scaling correctness.
- Use roughly 10–30 notes, a few longer notes and the smallest necessary boundary fixtures for ordinary-use acceptance. Large-collection benchmarks are not completion requirements. Historical measurements remain context; stress testing requires separate explicit approval.
- Keep basic multi-monitor correctness in scope: consistent monitor selection, geometry updates and notes remaining reachable after display changes. Automatic cursor-following is deferred.
- Target Wayland broadly. Hyprland is the current testing environment. Layer Shell is an actual implementation dependency, and compatibility with each desktop must be verified before support is claimed.
- Keep packaging, application icon, desktop metadata, licences and safe installation, upgrades and removal. Start with Arch/AUR and GitHub artifacts; additional formats are deferred.
- Extra Settings controls and manual drag ordering are removed from finishing work. Google Drive sync stays outside the first release and remains a future requirement pending a separate long-term scope decision. Native ports and scripting/MCP are outside this release.

## Status

| Category | Current position |
|---|---|
| Implemented and verified | The owner has accepted the UI design. Mechanical cleanup has passed formatting, all-target compilation, Clippy, build and static checks listed below. This establishes no new runtime results. |
| Implemented but awaiting verification | Current saving/formatting/history, recent menu/emoji/checklist fixes, focus, timeout, visibility, startup/shutdown, accessibility and scaling behavior. Earlier scoped regression results are historical evidence, not validation of the current checkout. |
| Confirmed defect | All Notes still processes complete bodies for short row excerpts. Historical stress measurements explain the follow-up, but current ordinary-use responsiveness remains unverified and does not require repeating the large collection. |
| Missing essential functionality | Consistent monitor selection and display-change recovery, distributable icon and release packaging. No explicit Layer Shell availability check is wired before surface initialization. |
| Optional future work | Automatic cursor-following, extra package formats and native ports. Drive is separately deferred, with its future requirement retained. |

## Cleanup boundary and verification

The approved cleanup removes unused resources and creation notifications, simplifies control types and identical theme branches, corrects stale comments/test expectations, completes CI dependencies and font notices, and updates current documentation. Monitor, performance, state and lifetime refactors are separate proposals.

No tests, benchmarks or app sessions are authorized for the agent in this cleanup. Allowed checks are formatting, locked offline compilation/build, Clippy with the existing GTK deprecation allowance, desktop-entry validation and static resource/reference checks. Test code may compile; it must not execute. The reviewed mechanical cleanup and documentation were committed locally as `8750f23`. The later historical removals are not yet committed. Further commits and pushes require explicit owner instruction.

Completed checks on the mechanical cleanup source at `8750f23`:

- `cargo fmt --all` and `cargo fmt --all -- --check` passed.
- `cargo check --locked --offline --all-targets` passed, compiling test code without running it.
- `cargo clippy --locked --offline --all-targets -- -D warnings -A deprecated` passed.
- `cargo build --locked --offline` passed with 24 existing GTK deprecation warnings.
- `desktop-file-validate data/dev.stickies.Stickies.desktop` and `git diff --check` passed.
- Static Python inspections resolved 37 manifest resources, all four font notices, current icon callers and the existing CSS loading path; checked local link existence; compared all 28 historical candidates with their pre-cleanup hashes; and confirmed only the eight approved runtime icons were deleted. That earlier link check did not establish availability in a fresh checkout. Stored-data, migration, formatting/history, shortcuts and stylesheet paths remain unchanged. The owner's two roadmap additions remain present, with Find's subsequent retirement clarified.

Check/build logs are under `/tmp/stickies-cleanup-{check,clippy,build}.log`. Clean-runner CI remains unverified. The agent ran no tests, benchmarks or app sessions.

The owner subsequently performed manual review. Private D-Bus launches logged accessibility-service activation failures; the normal desktop-bus run produced an empty `desktop-review.log` and the owner reported no warnings/errors. That supports a launch-environment cause, without establishing complete accessibility or checklist acceptance. Migration/failure fixtures, installed login/session-end, multi-monitor recovery and other compositor verification remain outstanding.

Current documentation no longer links to ignored/untracked audit files: useful findings and historical measurements are consolidated in the tracked roadmap. Git-aware static link inspection checks tracked destinations and heading anchors in the proposed contents, rather than local existence alone. No fresh checkout was created.

The owner subsequently approved all 28 original historical deletion candidates and the 2,848 images and 38 videos from five associated review galleries, and declined backups. All approved files were removed; retained reference indexes and galleries were updated. No deletion candidates remain pending from [this review](cleanup-review.md). Other gallery media, supplied reference images/specifications/SVG packs, runtime source/assets and saved notes remain unchanged. The later historical removals remain uncommitted.

## October 16 finishing tweaks

The four approved tweaks are implemented and remain uncommitted. Formatting, locked offline all-target compilation and Clippy with the existing deprecation allowance passed. Compilation reported the same 24 existing GTK deprecation warnings; Clippy reported no additional warnings. Static documentation links and whitespace were checked. Logs are `/tmp/stickies-tweaks-check.log` and `/tmp/stickies-tweaks-clippy.log`.

No tests, benchmarks or app sessions were run. The [verification checklist](manual-testing.md) records empty/single/archived-note boundaries, all creation paths and color persistence, repeated title/body emoji insertion with native remapping/focus/variants/history, and native search dismissal/placement/keyboard behavior. Include CLI/D-Bus search on both a running process and fresh startup because native popup grabs depend on desktop input delivery. Runtime correctness, visual placement and cross-compositor behavior remain unverified. Hover preferences, saved-data encoding, migrations, formatting and shortcuts are unchanged.
