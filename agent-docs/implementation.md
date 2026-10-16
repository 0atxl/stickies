# Stickies finishing roadmap

[current-milestone.md](current-milestone.md) defines current authorization. The owner accepts the implemented UI/UX. This roadmap tracks reliability and release work, without new UI design or extra controls.

## Product scope

Stickies is a small, native Linux scratchpad. It uses Rust, GTK4/libadwaita, Wayland Layer Shell, and SQLite. Local editing works offline without an account. There is no telemetry or application-owned cloud backend.

- Keep plain-text note bodies, clickable checklists, and the existing simple list controls. The October 12 feedback scope adds selected-text formatting as separate metadata, plus undo/redo and Find in note. Find was subsequently retired; body formatting and shared title/body undo/redo remain.
- Copy text is sufficient. Dedicated Markdown editing, Markdown rendering, and file export are not required.
- Basic multi-monitor correctness belongs in the first release. Automatic cursor-following is deferred; its previously agreed one-strip behavior remains future scope. Do not add a monitor picker or independently maintained decks per monitor.
- Google Drive sync remains a future requirement with simple setup and few controls, outside the first release. Optional enablement does not cancel that requirement. Its long-term scope needs a separate decision; no integration is selected and disconnected local editing must keep working.
- Application-level note encryption is not required. Authentication credentials for Drive still need appropriate storage.
- Target Wayland broadly, with staged compatibility verification and packaging/release work. Hyprland is the current testing environment, not the product boundary.
- Virgil is the default handwritten font, Inter the clean font, JetBrains Mono the monospace font, and Kalam an additional cursive choice. Reference images do not override these choices.

## Implemented work

The owner approved the October 16 finishing tweaks: a welcome message and hidden collection filters when no saved notes exist, varied persistent colors on creation, continued native emoji selection, and native outside-click dismissal for Quick Search. Hover-to-reveal stays unchanged. Native popup dismissal may consume the first outside click, a tradeoff the owner accepted. These changes require focused runtime verification in a new session; no tests or app sessions are authorized in this implementation task. The current checklist records the small boundary cases.

| Category | Evidence and limit |
|---|---|
| Implemented and verified | UI design accepted by the owner. Current static/compile checks belong in the milestone. Acceptance does not establish every runtime or compositor behavior. |
| Implemented but awaiting verification | Local SQLite notes and schema 1-4 upgrades; shared editing, formatting/history, lists, searches, archive/restore/Delete Undo; configuration, autostart, CLI/D-Bus actions, focus, timeout, motion and scaling behavior. Code and earlier scoped regression records exist; latest changes still need focused runtime acceptance. |

## Remaining first-release work

Complexity is Small, Medium or Large for the individual scope. Dependencies describe sequencing, not permission to implement or run tests.

### Ordinary-use verification scope

Use roughly 10–30 notes, mostly short, with a few longer notes containing realistic paragraphs, lists and formatting. Reuse that collection for editing, search, switching, startup and resource checks. Add only the smallest fixture needed for a specific boundary: an empty database, a single note, or just below/at/above the configured four- or five-slot capacity. Migration and failure fixtures need only the records and states relevant to that check.

Completion means reliable, responsive ordinary use. Record the environment and workload with any timing or resource observation; a few open/close cycles should settle without accumulating retained objects. Large collections, oversized bodies, endurance loops and stress benchmarks are outside default completion requirements and require separate explicit approval. Ordinary checks still require execution authorization under the current checklist.

| Item and category | Complexity and reason | Dependencies | User benefit | Completion condition |
|---|---|---|---|---|
| Saving, formatting/history and checklist crash regression. Implemented but awaiting verification. | Medium: shared buffers, hidden markers and save failure paths interact. | Existing storage/session/history regressions; separately authorized disposable verification. | Notes, marks and edits survive ordinary use and failures. | Current source passes focused shared-view, failure/retry, Unicode, history and checklist caret/crash checks without changing stored encoding. |
| Focus, timeout, shortcuts, accessibility and scaling. Implemented but awaiting verification. | Medium: input crosses native windows, popovers and layer surfaces. | Protocol availability and current manual checklist. | Existing controls remain usable with keyboard, assistive tools and different scales. | Layered Escape, docked-only inactivity, native menus, CLI forwarding, accessible names/focus and minimum-size behavior work at normal and fractional/high scales. |
| Startup/shutdown and installed login. Implemented but awaiting verification. | Medium: session delivery and executable discovery depend on the desktop. | Initial installation path; disposable session and write-failure fixtures. | Launch reliably and retain pending edits on delivered shutdown events. | An installed executable launches after login; delivered session-end/signals flush active sessions, log rejected writes and exit; invalid configuration remains untouched. |
| Full-body All Notes excerpts. Confirmed defect. | Small: bound the two label-input paths without truncating stored/editor text. | Choose a Unicode-safe excerpt bound; separately authorize focused ordinary-use verification. | Avoid laying out entire long bodies for a short list row. | Both row creation and live update bound excerpts; full-body search/edit/copy remain intact. All Notes opens and updates responsively with the ordinary collection and a few longer notes, including Unicode and line breaks. No large-collection benchmark is required. |
| Basic multi-monitor correctness. Owner now reports resolution recovery works; multi-monitor verification pending. | Medium: both layer surfaces, cached geometry and open editors must agree. | GDK monitor lifecycle and Layer Shell output selection; resolve current mismatch first. | Strip and notes remain reachable after plug/unplug, resolution and scale changes. | Surfaces and geometry use the same surviving output; hotplug and mixed-scale changes reclamp content without duplicate strips or lost edits. No automatic cursor following. |
| Cross-compositor Wayland compatibility. Protocol v4+ checks implemented; compositor verification pending. | Large: shell behavior differs despite shared protocols. | Layer Shell capability check; monitor fix; staged environments below. | Reach the intended Wayland audience with accurate support claims. | Record per-environment results and dependencies for placement, focus, input pass-through, resize/movement, fullscreen, scaling, hotplug and session startup; explain unsupported protocol environments. |
| GTK object lifetimes and final resource use. Implemented but awaiting verification. | Medium: source ownership review needs measured confirmation. | Separately authorized focused lifetime checks and ordinary-use observations. | Repeatedly opening notes/menus does not accumulate objects or stall the app. | With roughly 10–30 notes and a few longer bodies, closed views/menus release, startup/editing/switching remain responsive, and resource use settles after a few open/close cycles. Include both layer surfaces and supported display scales; record the environment and intervals. Historical stress results and numerical targets below are not release gates. |
| CI reproducibility. Implemented but awaiting verification. | Small: native dependency list and existing checks. | A clean runner with locked dependencies. | Builds work away from the development machine. | A separately authorized clean CI run installs native dependencies and completes the existing workflow. Local compilation alone is insufficient. |
| Initial packaging, icon and metadata. Missing essential functionality. | Medium: install paths, licences and data ownership need coordinated checks. | Stable runtime, CI and an approved application icon. | Users can install, launch, upgrade and remove Stickies safely. | Ship Arch/AUR and GitHub artifacts with desktop entry, icon, binary, font notices, source licence and checksums; verify install/login/upgrade/removal without deleting notes. |

## Focused follow-ups

These findings come from source inspection at `a2105f2` plus the mechanical cleanup. They are separate proposals, without implementation or execution authorization. Useful findings from the ignored local audit are consolidated here so current documentation does not depend on that file.

| Topic | Evidence and smallest follow-up |
|---|---|
| All Notes excerpts | `manager.rs::note_row` and the live update in `sync_editor` flatten complete bodies for ellipsized labels. Bound both inputs without truncating saved text or changing full-body search, editing or Copy text. The icon cache already exists. Use the ordinary workload above for current acceptance. |
| Monitor selection | Both layer windows now select one retained GDK monitor and use its logical geometry. Monitor removal, geometry and scale changes trigger recovery and placement clamps. No-output suspension preserves editing/Search state. Owner verification found stale resolution placement; the event-driven final-layout correction also failed the rebuilt-binary recheck. The owner subsequently reported resolution placement works. A temporary full-redraw launch improved Search. The owner then accepted fixed-size Search and confirmed no-match jitter was gone. Temporary diagnostics have been removed; the rendering root cause remains unestablished. Multi-monitor behavior and minimum-layout limits remain unverified. Cursor-following remains deferred. |
| Layer Shell availability | Startup checks `is_supported()` and requires `protocol_version() >= 4` before constructing surfaces. Missing/older support produces a stderr diagnostic. No fallback UI is provided; runtime rejection checks remain pending. |
| Application icon | The desktop entry names `dev.stickies.Stickies`, but no distributable application icon or package recipe exists. Keep that work in the initial release scope. |
| Duplicate state | `app.rs::AppState` retains an open-note guard and temporary overflow membership used by `main.rs`/`edge.rs`; returned events have no production consumers. Trace those responsibilities before proposing consolidation. Existing callers prevent treating the state model as dead code. |
| Checklist completion branches | `toggle_checklist` removes an existing checked/unchecked prefix itself and calls `toggle_line` only to add an absent prefix; other production callers pass bullet/numbered prefixes. The two checklist x/space swap branches have no production route. Only a formatting regression calls that old helper contract directly. Propose removing those branches and updating that regression to activate the actual embedded checkbox, preserving completion, history and spans. This is outside the mechanical cleanup. |
| GTK lifetimes and scaling | Old menu probes found strong callback cycles; current controls/editor/Sort callbacks use weak references, but complete release is unverified. Check editors, sessions, menus and buffer subscriptions with a few open/close cycles; fix demonstrated retention only. Two screen-sized surfaces are intentional for stable transitions. Observe ordinary use at supported scales before changing them; 36x36 icon masks also need a high-scale clarity check. |
| Checklist crash/caret | October 15, 02:19:07 IST, PID 1683626: GTK rejected byte index 29 beyond the visible line through `note_text::decorate` and `iter_at_location`, causing SIGABRT. Current code uses `line_at_y`, skips hidden markers/anchors and uses cursor geometry; checkbox toggles preserve selection and native edits advance a collapsed caret beyond hidden prefixes. The updated native regression compiled but was not run. Focus future checks on typing before anchors, after checkbox activation and around wrapped/empty lines. |
| Startup and graphics diagnostics | Installed login and delivered session-end/signals still need verification. Rejected shutdown writes log errors but do not prevent exit; crashes/SIGKILL cannot flush pending edits. Older pixman/invalid-rectangle warnings were transient and not reproduced later; retain stderr and investigate a reproducible trigger without claiming a fix. |
| Clean CI | Native dependencies and the GTK deprecation allowance were corrected. Neither a clean-runner workflow nor remote package installation was verified during cleanup; local compilation does not establish those results. |

### Historical performance context

The October 10 run used an isolated 1920x1080 output at 125%. Timing covered process launch to first mapped All Notes, including GTK/fonts/private-bus startup. These historical measurements explain the excerpt follow-up; they do not describe current performance or constitute a before/after benchmark on the owner's desktop.

| Collection | Largest body | First mapped All Notes | PSS |
|---|---:|---:|---:|
| 20 notes | 382 bytes | 1.33 s | 143.1 MiB |
| 1,284 notes | 382 bytes | 3.18 s | 178.2 MiB |
| 1,284 notes | 20,582 bytes | 54.18 s | 379.4 MiB |

The long-content run's final four one-second idle samples were 0% CPU after earlier settling activity; interaction/animation CPU acceptance was not established.

Earlier plans proposed at most 200 MiB PSS, interaction below 2% CPU, animation below 3%, exceptional one-second spikes at most 5%, and idle returning to 0%. Retain those as historical targets, not universal completion thresholds. No new stress measurements are required for release; stress testing needs its own explicit approval and agreed workload.

## Staged Wayland verification

1. Establish actual dependencies: Wayland, `zwlr_layer_shell_v1`, compatible GTK/libadwaita/gtk4-layer-shell libraries, a session bus for single-instance actions, and normal GTK toplevel move/resize support. Startup rejects missing Layer Shell or versions below v4 with a diagnostic; verify this path separately.
2. Use the existing Hyprland environment for the current regression baseline. Previous isolated Sway runs are useful scoped evidence, not a broad current support claim.
3. Verify Sway and KDE Plasma Wayland next. Include input/focus, fullscreen layer policy, minimum sizes, movement, login/session-end and hotplug/mixed scale. No development-machine window rule may be an installation prerequisite.
4. Expand to other available Layer Shell implementations, such as COSMIC, Niri, River and Wayfire, one environment at a time. Keep this compatibility work in scope; do not promise universal support based on protocol names alone.

The current edge implementation requires Layer Shell v4+ for OnDemand focus. GNOME Wayland without that protocol cannot provide this edge implementation; this is a compatibility limitation to document and review, not a Hyprland product restriction. X11 is outside the Wayland release. Omarchy-only DND is optional; other desktops currently leave the strip visible. Compositor keybinding examples stay in README.

Production uses GTK/Layer Shell rather than Hyprland/Sway IPC and does not modify compositor configuration. Separate-window floating placement is compositor-dependent and must not rely on the owner's window rules. Minimal System appearance reads GTK's dark preference/theme name. DND reads `OMARCHY_PATH` and `~/.local/state/omarchy/notifications.json`; fullscreen behavior selects Top/Overlay layers without a compositor-specific detector. Output selection and older protocol layer changes may remap surfaces, so verify focus and pending edits with the smallest useful display setup.

## Deferred work and explicit exclusions

| Item and category | Complexity and reason | Dependencies | User benefit | Completion condition if separately approved |
|---|---|---|---|---|
| Automatic cursor-following. Optional future work. | Large: global cursor access varies and moving surfaces can remap them. | Basic monitor correctness; agree how an open docked note behaves. | Convenience across monitors. | One strip follows the cursor without lost edits, duplicate surfaces or focus disruption on each supported environment. |
| Google Drive. Deferred future requirement, not part of the first release. | Large: authentication, offline edits, deletions and conflicts need preservation rules. | Separate long-term scope decision and feasibility work. | Notes available on multiple devices. | Select the smallest integration; connect/disconnect and reconcile initial/offline/concurrent edits and deletions without silently losing content. Do not copy a live SQLite database as the sync design. |
| AppImage, Flatpak, Debian and RPM. Optional future work. | Medium per format: runtime/portal and installation rules differ. | Working initial package and demonstrated distribution need. | More installation choices. | Each chosen format passes safe installation, upgrade/removal, startup and licence checks. |

Remove extra Settings additions and manual drag ordering from the finishing roadmap. Preserve existing preferences and controls. Windows/macOS ports and scripting/MCP remain outside this release. No rich document editor, attachments, team workspace, generic sync framework, application-owned backend, Markdown/export or note-encryption feature is planned.

## Constraints

Keep current module responsibilities and the Rust/GTK4/libadwaita/Layer Shell/SQLite stack. Preserve migrations, legacy configuration acceptance and saved styles. Keep MPL-2.0 and bundled asset licences. Do not introduce speculative storage, event or portability abstractions. Commit, push and test execution need their own authorization.
