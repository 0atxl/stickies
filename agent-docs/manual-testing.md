# Current verification checklist

The UI/UX is accepted. Verify correctness against [SPEC.md](SPEC.md) without redesigning it or restoring retired controls. This checklist requires separate execution authorization. No tests, benchmarks or app sessions run as part of the mechanical cleanup.

## Isolation and reporting

Use disposable XDG data/config/cache directories, an isolated Wayland display and a private session bus. Different XDG paths alone do not isolate a GApplication instance. Never use or alter the owner's saved notes or desktop configuration. Retain failure fixtures and stderr until investigated. Avoid historical review launchers that stamp schema 3 after applying all migrations.

For each authorized check, record the commit/diff, compositor and version, native library versions, display sizes/scales, fixture, result and remaining limitation. Distinguish source inspection, compilation, automated assertions, actual input and owner design acceptance. Earlier successful runs do not certify later changes. The tracked roadmap retains [focused findings](implementation.md#focused-follow-ups) and [historical performance context](implementation.md#historical-performance-context).

## Ordinary-use workload and boundaries

- Use roughly 10–30 notes, mostly short, with a few longer multi-paragraph notes. Include ordinary lists/formatting and a Unicode example in those notes; reuse the collection across checks.
- Add only the smallest necessary boundary fixture: an empty database, a single note, or one below/at/one above the configured four- or five-slot capacity. Use only the records/states needed for draft, archive/delete, migration and failure checks. Do not multiply all content variants, note counts and environments into an exhaustive matrix.
- Check ordinary startup, typing, switching, search and a few note/menu open/close cycles. Record workload, display setup and observation intervals for responsiveness/resource use; expect resources to settle and objects to release after closing views.
- Large collections, oversized bodies, endurance loops and stress benchmarks are excluded from default completion requirements. Historical measurements are context only. Stress testing requires separate explicit approval of its workload and execution; ordinary verification also remains subject to execution authorization.

## Notes, drafts and saving

- A fresh database stays empty. New drafts from either creation entry point stay out of tabs, markers, lists and search. Existing blank notes remain visible.
- The first successful nonblank title/body save publishes a draft in normal order; failed saves keep it hidden and open. Style/color/pin alone do not publish it.
- Closing the last view of a still-blank draft removes it. Another live view and pop-out transfer keep it available. Startup removes orphan drafts without removing old blank or published notes.
- Edit a note in All Notes and a separate window. Both share title/body, formatting, style and one save owner. Closing one view preserves pending work in the other; reopening an existing separate note focuses it.
- Verify autosave and immediate close/switch. In disposable failure fixtures, close, replacement, archive and Delete retain failed edits for retry. Copy text uses the latest body with textual checklist markers.
- Open schema 1/2/3 fixtures and retain note IDs, metadata, styles, archive/delete state and formatting through schema 4. Reject newer schemas without modification. Preserve legacy config aliases, old whole-note bold and font/canvas migration, including archived/deleted notes.

## Editing and history

- Use Virgil, Inter, JetBrains Mono and Kalam, S/M/L, every canvas pattern and the existing palette in both editors; verify live sharing and reopen persistence.
- Editable body selection replaces the fixed footer with B/I/U/S without changing its dimensions or moving text. Mixed selections show inactive marks. Hidden checklist prefixes/anchors do not determine selected mark state. Archived selections retain read-only controls. Titles remain plain text.
- Verify Ctrl+B/I/U, Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y, plus the existing native body/title right-click Undo/Redo entries and their availability. Continuous typing/deletion groups; pauses, newlines, direction/field changes, cursor movement, clipboard/drop, marks and list actions break groups. New edits clear redo. Unicode and partial formatting retain correct ranges.
- The footer Checklist control adds/removes checklist mode. Embedded checkboxes change completion, preserve caret/selection and persist markers. Enter continues lists or ends empty items; Backspace removes a checklist prefix. Bullet/numbered toggles preserve item formatting.
- Revisit the October 15 checklist crash through real typing before anchors, after checkbox activation, around hidden prefixes and wrapped/empty lines. Verify font-sized underline/strike and completed-item coverage without graphics assertions or SIGABRT.

## Collections and accepted controls

- A newly created All Notes window starts on Active, empty search and no selection. Refresh/sort/filter never choose a replacement automatically. Draft creation preserves rows and list selection until publication. Reopening an existing window retains its state.
- Search literal title/body text, overflow notes and Unicode case variants. Recent/Title sorting and persistent ordering Pin agree between All Notes and the strip. Individual Always open does not reorder notes.
- Archive/restore and Delete Undo preserve content, collection and pin state. Archived previews show read-only checkboxes; only Copy and Restore are enabled. Individual More has no Archive. All Notes More has canvas samples and Copy/Archive-or-Restore/Delete icons. Individual deletion shows the single replaceable six-second edge toast.
- Color is eight columns by two rows. Aa has four font samples and S/M/L. Canvas samples and individual list actions are in More. Footer menus switch on the first click, open inward on each edge and keep writing space fixed. Native emoji uses the adjacent compact chooser. Do not expect a floating selection bubble, whole-note weight control, Find, previews or title tooltips.

## Edge, focus and timeout

- Check Left/Right vertical titles and Top/Bottom horizontal titles, Classic/Stacked, Pills/Bar and four/five slots, including few/empty notes and long titles. Strip order follows the active sort and excludes drafts/archived/deleted notes.
- Verify the 140 ms reveal dwell, 350 ms leave grace, common 160/110 ms reveal/collapse clock and current attached hover behavior. Reduced/system-disabled motion settles immediately. Geometry and input bounds remain stable through reversals, switches and fades.
- Search stays open after focus loss/pointer departure. Its hidden scrollbar still permits wheel/arrow access to every result; Enter opens the selected note or creates from a nonblank unmatched title. Empty query never creates. Escape/control actions dismiss Search explicitly.
- Clicking another tab saves and switches directly. The sheet crossfades and moves only as needed for its arrow. Manual Close reveals tabs; a successful inactivity close collapses to the marker. Failed saves retain editor and tabs.
- Escape dismisses footer popovers or clears body selection before closing an individual note. Check the native context menu/emoji handoff, keyboard focus transfer and transparent-area click-through.
- Off/1/2/5-minute choices persist. Only docked notes time out. Pointer-enter/motion, keys, clicks, scroll and controls restart activity. Always open cancels the timer; turning it off or changing the interval starts the appropriate fresh interval. Separate windows and All Notes never auto-close.
- Presets persist and Reposition remains preview-only until Done. Cancel, Escape, DND and All Notes closure discard the preview. Failed Done retains it; failed note saves block entry. Reject Settings writes and confirm accepted control state returns without repeated submissions.

## Wayland, accessibility and release

- Follow the staged environments in [implementation.md](implementation.md). Record Layer Shell availability/version and native dependencies. Verify unsupported-protocol startup handling after its separate implementation. Protocol support alone is not app support.
- Verify both layer surfaces and geometry use the same output after the monitor follow-up. Cover laptop-only, external-only, hotplug, resolution changes and mixed scale; notes and controls must stay reachable without lost edits or duplicate strips. Cursor-following is deferred.
- Verify current focus, toplevel movement/resizing and default/minimum sizes at normal, fractional and high scale. Separate-note floating must not depend on the development machine's rules. Test fullscreen Top/Overlay policy. Omarchy DND is the only current provider; other desktops leave the strip visible.
- Check keyboard traversal, actual accessible names/roles/states and readable focus in editors, Settings, menus and Search. No design change is implied by a verification request.
- Verify one running instance and all four CLI/D-Bus actions through the README commands. Repeated Search retains its query; actions respect save guards and Toggle strip cancels Reposition.
- Validate partial/invalid config, failed writes, autostart reconciliation and an actual installed login. In an isolated session, deliver session-end and catchable shutdown signals with pending edits; inject a rejected write and inspect per-note errors and other-session saves. Crashes/SIGKILL cannot flush pending edits.
- The existing `tests/probe_shutdown.py` requires a disposable Wayland session, private bus and `STICKIES_TEST_SANDBOX=1`. It is useful regression tooling, but is not run or newly validated by this cleanup.
- When separately authorized, observe both screen-sized surfaces with the ordinary-use collection above during startup, editing, animation and idle. Check a few open/close cycles for retained objects and resource growth, including supported scaling. Record the environment and intervals; old large-collection measurements and CPU/PSS targets are historical context, not completion gates.
- On a clean runner, complete the existing CI workflow. Verify initial package install, icon/desktop entry, licences, login, upgrade and removal without deleting notes.
