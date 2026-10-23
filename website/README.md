# Stickies launch page

This is a standalone static site. `index.html`, `styles.css`, `app.js` and `assets/` can be served together from any static host, including a GitHub Pages project subpath. No dependencies or build step are needed. The repository includes its release-update workflow; hosting has not been configured and nothing has been deployed.

After browser verification is authorized, open `index.html` locally or serve this directory over localhost. Copy text needs a browser that allows clipboard writes in that context. Failure produces visible feedback; no clipboard-read permission is requested.

## Release and appearance

On October 23, 2026, GitHub's public latest-release API confirmed [v0.10.1](https://github.com/0atxl/stickies/releases/tag/v0.10.1), neither a draft nor a prerelease. Its source commit is `f4b70bac936c402aac9418988f8fdf84dc601743`, with package version `0.10.1`. The updater applied that actual metadata to the page, including the API-returned URLs for `stickies-windows-x86_64.zip`, `stickies-0.10.1-1-x86_64.pkg.tar.zst`, and `SHA256SUMS`. No required release assets are missing. File sizes are omitted.

The demo depicts the optional Playful Clean appearance and Classic connector shipped in v0.10.1. Its disclosure now reads "Simplified demo of Playful Clean. Downloads are v0.10.1." Wavy remains the application's default. The browser demo's editing and layout simplifications are listed below.

The `data-demo-since="v0.10.1"` attribute on the disclosure records the first release containing this appearance. It is an explicit editorial choice. When the demo changes to depict unpublished UI, set it to `unreleased`; set it to the new tag only after confirming that release contains the depicted appearance. The updater retains the preview warning until the baseline is satisfied.

Visual references are `src/appearance.rs`, `src/note_controls.rs`, `src/edge.rs`, `src/app.rs`, `assets/edge-strip.css` and `agent-docs/SPEC.md`. The app's icon, Playful control icons, Inter and Virgil are copied unchanged. Font licenses remain beside their fonts. The app icon also supplies the favicon and social image. Social metadata uses its existing public, commit-pinned image URL, so it does not invent a deployment origin. Website code uses the repository's MPL-2.0 license, copied here for standalone hosting.

## Automatic release updates and Vercel

`.github/workflows/website-release.yml` runs on published stable releases, prereleases promoted to stable, or a manual Actions run. Drafts and prereleases do not update the page. It checks out the current default branch and reads `/repos/0atxl/stickies/releases/latest`, so rerunning an old release job still uses the latest public stable release.

`scripts/update-website-release.py` updates only the marked release fields in `website/index.html`: version labels, Windows/Arch download URLs, Arch install command, release-note/checksum/source links and the demo disclosure. It requires one uploaded, nonempty Windows ZIP, matching-version Arch x86_64 package and checksum file. Missing assets, unexpected URLs or changed HTML markers stop the update before writing the page. Actual checksum values are not downloaded or validated by this updater; it verifies the metadata and links only.

The workflow uses its built-in `GITHUB_TOKEN` with `contents: write`, commits as `github-actions[bot]` and pushes only the changed page to the default branch. It makes no commit when the page is already current. Push conflicts or branch rules that disallow direct writes fail visibly; the workflow never force-pushes or changes branch protection. Built-in-token pushes do not start the repository's separate CI/Packages workflows. This workflow contains no tests or package builds.

To activate:

1. Ensure the website, updater and workflow are on the default branch. The owner authorized their initial commit and push on October 23. The workflow must be available on the default branch for manual runs.
2. In Vercel, import `0atxl/stickies` and set Root Directory to `website`, Framework Preset to Other, Build Command override to empty, Output Directory to `.`, and Production Branch to the repository's default branch, currently `main`. No environment variables or Vercel secrets are needed for this Git integration.
3. Vercel deploys website commits through its GitHub connection. Future stable releases with their assets already attached update the page and create such a commit automatically. The first Vercel deployment will use the already-updated v0.10.1 page.

If assets are attached after publication, upload them and rerun **Update website release** from Actions. No-op runs create no Vercel deployment. Releases published using another workflow's `GITHUB_TOKEN` do not trigger additional release workflows; that publishing workflow must explicitly dispatch this workflow after uploading assets, or the owner can run it manually. Human publication through GitHub or a separately authenticated CLI triggers the release event normally.

For a local update using actual metadata, download the public `/releases/latest` response to a file and run `python3 scripts/update-website-release.py --release-json /path/to/release.json` from the repository. This performs the update; it does not commit, push, publish or run tests.

References: [GitHub release triggers](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#release), [GITHUB_TOKEN event behavior](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow#triggering-a-workflow-from-a-workflow), [Vercel Git deployment](https://vercel.com/docs/git), and [Vercel static build settings](https://vercel.com/docs/builds/configure-a-build).

## Demo behavior and simplifications

- The demo begins with a marker. Hover reveals three tabs; click, tap or keyboard activation does the same. Tab buttons open notes. Close returns focus to the selected tab. Escape closes a popover first, then a note, then the tabs, only while focus is inside the preview.
- Titles, text, checks, color and font changes remain in memory when switching notes. Reset or refresh restores the samples. The page has no content storage, cookies, analytics, backend or content transmission.
- Editing uses one textarea per logical line. Enter splits a line and continues its checklist. Enter on an empty checklist item removes the checkbox. Backspace at the start removes checklist mode or joins a plain line to the preceding one. Shift+Enter and pasted line breaks stay inside the current item. The checklist control toggles the current or most recently focused line. Browser-native text editing is retained, but there is no full-note undo history or rich-text formatting.
- The palette offers four actual app colors. Font choices are Virgil and Inter. The footer preserves Color, Aa, Checklist, Always open and More in that order. It omits native font sizes, other fonts, full palette, formatting, canvas styles and window management.
- Always open disables a one-minute docked inactivity timeout. It applies to the open view and resets when closing or switching notes. It never changes note order. Pointer departure alone does not close an open note. Activity inside the preview restarts the idle interval.
- More offers Copy text, including textual checklist markers, and Reset demo. Copy happens only on explicit activation. Clipboard failure is announced and shown. No clipboard-read, filesystem, desktop or notification access is requested.
- The bounded preview uses one right edge. Tabs retain 104px lengths, 8px gaps and 6px hover movement. Hit areas and note geometry adapt to the browser instead of recreating native multi-window placement. Native strip utility actions are omitted. Motion uses the app's 160ms reveal, 110ms collapse and 120ms note timing, with reduced-motion overrides.

## Verification

Static inspection covers release metadata, appearance differences, copied-asset provenance, in-memory state, text insertion through DOM values, focus handling and responsive CSS. JavaScript syntax, HTML references, local asset paths, release-asset URLs and whitespace are checked without executing the demo.

The automation addition was checked through Python syntax parsing, workflow YAML parsing, HTML marker/reference inspection and whitespace checks. The real v0.10.1 API response was used to update the page. The hosted workflow, bot push and Vercel deployment have not been exercised.

No automated tests, browser/app sessions, screenshots, benchmarks or Lighthouse runs were authorized or performed. Rendering and runtime behavior remain unverified. The initial website commit uses `[skip ci]` to preserve that test restriction. Vercel hosting setup remains with the owner.

Suggested checks for separate authorization:

- Keyboard-only reveal, edit, switch, footer controls, popover dismissal, Escape and leaving the demo with Tab. Check long titles, multiline text, reset and the inactivity timer with Always open on and off.
- Touch reveal, note switching, checkbox hit areas, editing with the on-screen keyboard, and both download choices.
- Reduced-motion preference, narrow layouts at 320px and 390px, tablet/desktop layouts and browser zoom. Confirm the note footer and text remain usable without horizontal page overflow.
- Follow Windows, Arch, AUR, source-installation, release-note and checksum links. Check clipboard success/failure on the intended host. Confirm the upcoming-appearance label remains accurate before publishing.
- Verify stable/prerelease handling, incomplete assets leaving the page intact, repeated runs producing no commit, and the complete release-to-Vercel path after setup. These automated/hosted verification runs require separate authorization.
