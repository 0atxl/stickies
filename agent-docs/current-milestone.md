# Current milestone

The owner accepts the current UI/UX and design choices. UI design is complete. The active work is reliability, Wayland compatibility and finishing a simple sticky-notes app. Remaining behavioral verification does not reopen design decisions.

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

No tests, benchmarks or app sessions are authorized for the agent in this cleanup. Allowed checks are formatting, locked offline compilation/build, Clippy with the existing GTK deprecation allowance, desktop-entry validation and static resource/reference checks. Test code may compile; it must not execute. The owner has authorized a local commit of the reviewed cleanup and documentation. Push remains unauthorized.

Completed checks on the final source:

- `cargo fmt --all` and `cargo fmt --all -- --check` passed.
- `cargo check --locked --offline --all-targets` passed, compiling test code without running it.
- `cargo clippy --locked --offline --all-targets -- -D warnings -A deprecated` passed.
- `cargo build --locked --offline` passed with 24 existing GTK deprecation warnings.
- `desktop-file-validate data/dev.stickies.Stickies.desktop` and `git diff --check` passed.
- Static Python inspections resolved 37 manifest resources, all four font notices, current icon callers and the existing CSS loading path; checked local link existence; compared all 28 historical candidates with their pre-cleanup hashes; and confirmed only the eight approved runtime icons were deleted. That earlier link check did not establish availability in a fresh checkout. Stored-data, migration, formatting/history, shortcuts and stylesheet paths remain unchanged. The owner's two roadmap additions remain present, with Find's subsequent retirement clarified.

Check/build logs are under `/tmp/stickies-cleanup-{check,clippy,build}.log`. Clean-runner CI remains unverified. The agent ran no tests, benchmarks or app sessions.

The owner subsequently performed manual review. Private D-Bus launches logged accessibility-service activation failures; the normal desktop-bus run produced an empty `desktop-review.log` and the owner reported no warnings/errors. That supports a launch-environment cause, without establishing complete accessibility or checklist acceptance. Migration/failure fixtures, installed login/session-end, multi-monitor recovery and other compositor verification remain outstanding.

Current documentation no longer links to ignored/untracked audit files: useful findings and historical measurements are consolidated in the tracked roadmap. Git-aware static link inspection checks tracked destinations and heading anchors in the proposed contents, rather than local existence alone. No fresh checkout was created.

Historical document, screenshot and tooling deletion still requires approval of the exact list in [cleanup-review.md](cleanup-review.md), included as a tracked current document with this commit. All 28 historical candidates remain in place, including ignored/untracked material. No historical files were moved into HISTORY.md.
