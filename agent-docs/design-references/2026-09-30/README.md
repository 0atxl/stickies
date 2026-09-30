# UI design references, September 30, 2026

These are unchanged copies of the files supplied by the user from `/home/rinzler/Downloads`. All nine images, the original UI/UX document, and the two latest supplied SVG packs are preserved here. The copies were compared byte for byte with their source files.

Use [the UI implementation plan](../../ui-implementation-plan.md) for the decisions made after the document was written. The user's later clarifications supersede conflicting text in the original document and details in example images. The document's implementation recipes do not authorize code changes or architectural rewrites.

## Original document

- [stickies-uiux-spec.md](stickies-uiux-spec.md)

Keep this copy unchanged so the original can be compared with the agreed implementation plan.

## Images

| Reference | File |
|---|---|
| Minimal Dark individual note | [minimal black individual sn.png](minimal%20black%20individual%20sn.png) |
| Minimal Light individual note | [minimal white individual sn.png](minimal%20white%20individual%20sn.png) |
| Playful individual note | [playful individual sn.png](playful%20individual%20sn.png) |
| Paper individual note | [paper indvidual sn.png](paper%20indvidual%20sn.png) |
| Compact note tools | [tools.png](tools.png) |
| Edge dots, tabs, search, and open note | [edge strip assest.png](edge%20strip%20assest.png) |
| Minimal Light All Notes | [minimal white main app.png](minimal%20white%20main%20app.png) |
| Playful All Notes | [playful main app.png](playful%20main%20app.png) |
| Paper All Notes | [paper main app.png](paper%20main%20app.png) |

Minimal Dark All Notes uses the Minimal Light layout with dark colors. The title underlines in the images are examples, not an instruction to underline every title. Image proportions and lettering do not determine final text sizes or replace the chosen fonts.

## Earlier SVG packs

- [Updated base pack](stickies-icons%20%281%29.zip), 20 SVGs. This includes the earlier base pack's 18 icons plus the grip and tag dot.
- [Playful pack](stickies-icons-playful.zip), 20 SVGs.

The app currently bundles 13 base and 13 Playful SVGs. Both preserved packs contain the older palette, sort, and infinity icons. They do not contain the document's `text-style.svg`, `color-swatch.svg`, or `filter-sliders.svg`, or the four-square All Notes icon shown in the edge reference. Add or replace only the assets used by the agreed controls. The Playful outline pin also changes geometry slightly, despite the document's rule that only stroke weight changes.

The separate `wobbly_border.rs` reference was not located. The user agreed that the existing note-ID-seeded Paper tear and Playful wave drawing can be retained, so the missing file does not block this plan.

This reference set and the selected planning documents are explicitly tracked for the user's requested checkpoint commit. Other local agent documents follow the project's existing Git ignore rule for `agent-docs`.
