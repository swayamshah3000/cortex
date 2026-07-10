---
phase: 15-visual-intelligence-rupixel
status: skipped_with_deviation
verified: 2026-07-08
score: 0/? (phase de-scoped)
must_haves_verified: 0
must_haves_deferred: all
---

# Phase 15 Verification — Deviation Notice

## Verdict

**Status: SKIPPED — rupixel scope not aligned w/ v1.1 milestone budget.**

Prior audit (Phase 8 discuss) confirmed rupixel does **CLIP visual search over page screenshots**, not entity extraction or lightweight thumbnails. To integrate rupixel:
1. Add PDF/image → screenshot pipeline (headless browser or PDF renderer)
2. Encode screenshots via CLIP (sidecar Node process or ORT + CLIP ONNX model)
3. Store visual embeddings alongside HNSW text embeddings
4. Add visual-search IPC + UI

Estimated ~800 LoC + a real image model dependency (~200MB CLIP-ViT). High cost for v1.1.

**Cheap alternative attempted:** Just render PDF page 1 + JPG/PNG originals as thumbnails. Even this requires a Rust PDF renderer (pdfium-render) + thumbnail cache. ~400 LoC for a polish feature that doesn't unlock new capabilities.

**In autopilot mode with "don't waste tokens" directive: skipped for v1.1. Reopen in v1.2 as "Visual Search" — a marketable differentiator worth its own release.**

## Impact

- **Current placeholder unchanged.** `Document.thumbnail_color` continues to render a hex-color chip on cards. Users don't see actual document previews on cards (but can see them via the file preview built in Phase 6).
- **Phase 6 file preview still works** — click a doc, see the full PDF/image content. No regression.

## Deviation Accepted

v1.1 ships with the intelligence stack (LLM entities, LLM space labels, hierarchical spaces, entity navigation, saved searches, related docs). Visual intelligence is a big enough feature to be its own release.

## Next

All non-lifecycle phases resolved. Continue to milestone lifecycle: audit → complete → cleanup.
