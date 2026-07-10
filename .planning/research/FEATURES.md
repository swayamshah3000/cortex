# Feature Research

**Domain:** Desktop document intelligence / personal document organizer with AI auto-categorization
**Researched:** 2026-02-27
**Confidence:** MEDIUM-HIGH (core table stakes HIGH from multiple sources; differentiators MEDIUM from market analysis and competitor research)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete or broken. No credit for having them; significant penalty for missing.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Full-text search with keyword matching | Every file manager since 1995 has search. Spotlight normalized it on macOS. | LOW | Baseline before any AI features. Must be instant (<100ms UX feel). |
| Search result highlighting | Users need to see WHY a result matched. Without excerpts, search feels like guessing. | LOW | Show matched snippet with surrounding context. Critical for trust. |
| Watched folder monitoring | The whole value prop is automatic org. Requiring manual import contradicts it. | MEDIUM | File watcher with debounce. notify-rs handles this. |
| Document preview (PDF, DOCX, images) | macOS Quick Look and every DMS sets expectation of in-app preview. | MEDIUM | react-pdf for PDFs. DOCX conversion to HTML or render natively. No downloading required. |
| File type icons and visual differentiation | Users scan visually. A wall of identical icons breaks comprehension. | LOW | Already in design spec (Lucide icons per extension). Implemented. |
| Tag system (manual tagging) | Every modern file system (Finder tags, Notion, etc.) has tagging. | LOW | User-created tags as a baseline. Auto-tags are the upgrade. |
| Favorites / starred documents | This is muscle memory from every email client, file manager, browser. | LOW | Heart/star toggle. Simple persistence. Already in spec. |
| Document metadata display | Users need to know: what is this, how big, when modified, where on disk. | LOW | Name, path, size, dates, file type. Already in DocumentMeta sidebar. |
| Recent documents view | "What was I just working on?" is the most common document access pattern. | LOW | Time-sorted list grouped by Today/Yesterday/This Week. In spec. |
| Basic filtering (by type, date, size) | Users who can't find via search need to narrow by filter. Expected in any search UI. | LOW | Filter chips pattern already in design. Apply pre-query in ruvector-filter. |
| Settings persistence | Users will configure embedding model, watched folders, exclusions. Must survive restarts. | LOW | Serialize to JSON in app data dir. Standard Tauri pattern. |
| Onboarding flow | Cold start without explanation causes immediate abandonment. | MEDIUM | 4-step wizard (Welcome, Select Folders, Scanning, Ready). In spec. |
| Background indexing with progress indicator | If indexing blocks UI or is silent, users assume it's broken. | MEDIUM | System tray indicator + TopBar indexing progress. In spec. |
| Exclusion patterns (node_modules, .git, etc.) | Power users will immediately need to exclude build artifacts and hidden dirs. | LOW | Pattern matching in file watcher config. |
| Document re-indexing on change | If a document changes and search shows stale content, trust is destroyed. | MEDIUM | Content hash in metadata. Re-index on notify-rs Modified event. |

### Differentiators (Competitive Advantage)

Features that set Cortex apart from Spotlight, DEVONthink, Finder, and generic DMS tools. These are the "Find anything. Organize nothing." promise made real.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Semantic / natural language search | "Property tax documents from last year" returns the right files even if they don't contain those exact words. Zero competitors do this locally. Spotlight is keyword only. | HIGH | Core differentiator. ONNX + HNSW via RuVector. Already in architecture. |
| Auto-generated Smart Spaces (GNN clustering) | Documents self-organize without any user action. DEVONthink's "Classify" is suggestion-based and manual. Cortex makes it automatic. | HIGH | GNN clustering via ruvector-gnn. The defining feature of the product. |
| "See why" explanations for space membership | Users are skeptical of AI auto-organization. Showing "This is in Work because it shares entities with 14 other work contracts" builds trust and makes the AI legible. | MEDIUM | Extract shared tags, entities, top similar docs. Display in Space detail sidebar. |
| Entity extraction surface (people, orgs, amounts, dates) | Searching "Allstate invoice over $500" is impossible without entity extraction. This is a step above keyword or semantic. | HIGH | NER pipeline in Rust. On-device. Already in architecture. Requires NLP library (e.g., nlp-rust or ONNX NER model). |
| Space network graph visualization | Shows relationships between spaces and documents as a graph. No personal document organizer does this. Satisfies power users and is compelling in demos. | HIGH | react-force-graph or Sigma.js. ruvector-graph provides the data. Already in Insights route spec. |
| Related documents discovery | "See Also" from DEVONthink is their most-praised feature. Cortex can do this automatically without training the AI. | MEDIUM | ruvector-graph edges. Surface in document detail sidebar. |
| Self-learning ranking (SONA engine) | Search results improve the more you use it. No static ranking. Click-through data tunes results. No personal tool does this today. | HIGH | ruvector-attention + SONA. Already in architecture. |
| Local-first privacy guarantee | Windows Recall backlash proved that users fear AI indexing their private files. Cortex's local-only guarantee is a trust differentiator, especially for sensitive documents (medical, legal, financial). | LOW (marketing), MEDIUM (enforcement) | Must be communicated prominently in onboarding. Never send content to cloud by default. Opt-in only for API embeddings. |
| Command palette (Cmd+K) universal search | Power users live in command palettes (Alfred, Raycast, Linear). Bringing this UX to document management is unexpected and valued. | MEDIUM | Already in spec. Keyboard-first navigation across all routes. |
| Sub-space hierarchy | Documents can belong to nested categories (Work > Legal > Contracts). DEVONthink has nested groups; smart auto-nesting is novel. | MEDIUM | ruvector-hyperbolic-hnsw for hierarchy-aware clustering. Already in data model. |
| Multi-format document preview without leaving app | Users currently bounce between Finder, Preview, Word, Excel. Unifying preview in one app removes friction. | MEDIUM | react-pdf (PDF), DOCX conversion, image native, markdown render. |
| Incremental search-as-you-type | Instant results as user types (not wait-for-Enter). Feels like magic compared to legacy DMS. | MEDIUM | Debounce 150ms + streaming results from HNSW. |
| Analytics dashboard (document activity, space evolution) | Shows users their own information landscape. No consumer tool does this. | MEDIUM | Recharts. Donut (by type), area (indexed over time), bar (by space). In Insights spec. |
| Domain expansion (transfer learning for new spaces) | When a new category of documents arrives, Cortex bootstraps from related existing spaces instead of starting cold. | HIGH | ruvector-domain-expansion. Novel capability beyond any consumer tool. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good on the surface but introduce complexity, trust issues, or scope creep that outweighs value.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Auto-move files on disk | "Organize my files automatically" sounds like the dream | Windows Recall backlash showed users viscerally reject AI touching their files. Moving files breaks shell scripts, symlinks, other app references. If the AI is wrong once, user loses the file. Trust destroyed permanently. | Virtual Smart Spaces only. Files never move. Spaces are views over the real filesystem, not relocations. |
| Real-time collaboration / sharing | Teams want to share organized document spaces | Out of scope (stated in PROJECT.md). Adds server complexity, auth, sync conflicts. Not the product. | Single-user local-first. Share individual files via OS share sheet. |
| Cloud sync of document index | "I want this on all my devices" | Breaks local-first guarantee. Adds cloud dependency. Complex conflict resolution. Significant scope expansion. | Offer export/import of the index database as a future v2 feature. |
| AI chat / Q&A over documents | "Ask questions about my documents" (RAG chatbot) | Sounds compelling but is a separate product surface. Requires LLM integration, context window management, hallucination mitigation. Blurs focus. Users expect it to work perfectly from day one. | Surface document excerpts and entity extraction instead. Smart search already answers "find the document that says X." |
| Version control / document history | "Track changes to my documents" | Git for documents is an unsolved UX problem. Requires copying file content on each change. Storage explodes. Conflicts with "read-only" local-first posture. | Track indexing timestamp, surface modified_at prominently. Show re-indexing events in activity feed. |
| Email indexing / IMAP integration | Power users want email alongside documents | Massively increases scope and attack surface. Email authentication, OAuth flows, IMAP protocol complexity. DEVONthink took years to get this right. | Encourage exporting emails to PDF/EML and dropping into watched folders manually. |
| Browser history / bookmark indexing | "All my knowledge in one place" | Screen capture (Recall) backlash shows users don't trust AI with browsing history. High privacy risk. Massive scope expansion. | Watched folder for ~/Downloads already catches most web-originated documents. |
| Mandatory cloud embeddings for initial setup | Faster, higher-quality embeddings via API | Breaks local-first trust guarantee. Creates dependency. Users in air-gapped/corporate environments can't use it. | Local ONNX default always. Cloud embeddings strictly opt-in in Settings > AI & Models. |
| Automatic smart folder renaming | "AI should name my spaces better as docs change" | Unexpected name changes break user's mental model. Users build habits around space names. | Suggest renames via notification: "Your 'Misc' space now has 30 documents about insurance. Rename to 'Insurance'?" |
| Full OCR on all images by default | Comprehensive indexing | OCR is slow (500ms–2s per image). Running on thousands of images in watched folders will cause fan noise, battery drain, CPU spike during initial indexing. Users will complain. | OCR opt-in per folder or file type. Or run OCR in lowest-priority background thread with progress indicator. |

---

## Feature Dependencies

```
[Watched Folder Monitoring]
    └──enables──> [File Change Detection]
                      └──enables──> [Document Parsing]
                                        └──enables──> [Embedding Generation]
                                                           └──enables──> [Vector Storage (HNSW)]
                                                                             ├──enables──> [Semantic Search]
                                                                             ├──enables──> [GNN Clustering]
                                                                             │                 └──enables──> [Smart Spaces]
                                                                             │                                   └──enables──> [Space Detail View]
                                                                             └──enables──> [Related Documents]

[Entity Extraction]
    └──enhances──> [Semantic Search] (structured entity filters on top of vector search)
    └──enables──> [Entity-based Search Filters]

[GNN Clustering]
    └──enables──> [Smart Spaces]
                      └──enables──> [Space Network Graph]
                      └──enables──> [Sub-space Hierarchy]
                      └──enables──> [Domain Expansion]

[SONA Self-Learning]
    └──requires──> [Semantic Search] (generates learning signals from queries)
    └──enhances──> [Search Result Ranking] (improves over time)

[Content Hash]
    └──enables──> [Change Detection]
                      └──enables──> [Re-indexing on Document Change]

[Metadata Filtering]
    └──enhances──> [Semantic Search] (hybrid: structured pre-filter + vector search)

[Tag System (manual)]
    └──enhances──> [Tag System (auto-generated)]
    └──enhances──> [Metadata Filtering]

[Onboarding Wizard]
    └──requires──> [Watched Folder Monitoring] (step 2: select folders)
    └──requires──> [Background Indexing] (step 3: scanning progress)
    └──requires──> [Smart Spaces] (step 4: spaces ready)
```

### Dependency Notes

- **Embedding Generation requires Watched Folder Monitoring:** The indexing pipeline starts with file detection. No file detection = nothing to embed.
- **Smart Spaces requires GNN Clustering requires Vector Storage:** GNN operates on stored embeddings. Cannot cluster what hasn't been embedded. Minimum corpus size for meaningful clusters is ~20-50 documents.
- **SONA Self-Learning requires Semantic Search:** SONA learns from search interactions. No search = no learning signals. This feature has zero value on a cold start.
- **Entity Extraction enhances Semantic Search:** Entity extraction can run independently, but its value is realized when search can filter by extracted entities. Build extraction before wiring it into search filters.
- **Domain Expansion requires existing Smart Spaces:** Transfer learning bootstraps from existing cluster centroids. Requires established spaces first. This is a v1.x feature, not MVP.
- **Sub-space Hierarchy conflicts with flat auto-clustering:** GNN outputs flat clusters initially. Sub-space hierarchy requires hierarchical clustering (ruvector-hyperbolic-hnsw). Don't promise sub-spaces in MVP unless hyperbolic HNSW is confirmed working.
- **OCR conflicts with initial indexing speed:** Running OCR on all images during initial folder scan will block the "Spaces Ready" milestone for large collections. Make OCR async and explicitly backgrounded.

---

## MVP Definition

### Launch With (v1)

Minimum viable product — validates the "Find anything. Organize nothing." promise.

- [ ] Watched folder setup + file change monitoring — without this nothing works
- [ ] Document parsing: PDF, DOCX, TXT, MD (the 90% use case; OCR and spreadsheets can wait)
- [ ] Local ONNX embedding generation (all-MiniLM-L6-v2) — local-first from day one
- [ ] Vector storage and HNSW indexing via RuVector — enables all search and clustering
- [ ] Semantic search with natural language queries — the #1 differentiating feature
- [ ] Search result highlighting with excerpts — table stakes for trust
- [ ] GNN clustering → Smart Spaces (auto-generated) — the defining feature
- [ ] Space view with document list — users need to see what's in each space
- [ ] Background indexing with progress indicator — prevents "is it broken?" perception
- [ ] Basic metadata filtering (type, date) in search — needed for precision
- [ ] Tag system (auto-generated from content) — complements spaces
- [ ] Favorites system — simple, high-value, low-cost
- [ ] Recent documents timeline — most frequent access pattern
- [ ] Document detail: preview + metadata sidebar — for quick verification without leaving app
- [ ] Onboarding wizard — reduces cold-start abandonment
- [ ] Settings: watched folders, embedding model toggle, exclusion patterns

### Add After Validation (v1.x)

Features to add once core indexing and search are working and users are finding value.

- [ ] Entity extraction (dates, amounts, people, organizations) — enhances search precision significantly
- [ ] Entity-filtered search ("invoices over $500") — requires entity extraction
- [ ] Related documents panel in document detail — needs graph edges populated
- [ ] Space network graph visualization — needs populated graph; compelling demo feature
- [ ] SONA self-learning (click-through tuning) — needs a base of search interactions to learn from
- [ ] Manual tagging (user-created tags) — users will request this after auto-tags prove useful
- [ ] OCR for images (opt-in per folder) — needs to be backgrounded; don't default on
- [ ] Spreadsheet indexing (XLSX, CSV via calamine) — less common file type, add after PDF/DOCX

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] Sub-space hierarchy (hyperbolic HNSW clustering) — complex; requires validated flat spaces first
- [ ] Domain expansion (transfer learning for new spaces) — advanced capability; needs mature cluster graph
- [ ] Optional cloud embeddings (OpenAI text-embedding-3-small) — opt-in upgrade path for quality
- [ ] Space naming via LLM (Ollama local) — nice-to-have; rule-based naming works for MVP
- [ ] Analytics / Insights page (full charts and network graph) — polish feature, not core utility
- [ ] Command palette (Cmd+K) with full navigation — great UX but not blocking utility
- [ ] Keyboard shortcuts (Cmd+1/2/3 etc.) — power user delight, post-v1

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Watched folder monitoring | HIGH | MEDIUM | P1 |
| Document parsing (PDF, DOCX, TXT) | HIGH | MEDIUM | P1 |
| Local ONNX embedding | HIGH | MEDIUM | P1 |
| Semantic search | HIGH | HIGH | P1 |
| Search highlighting / excerpts | HIGH | LOW | P1 |
| GNN clustering → Smart Spaces | HIGH | HIGH | P1 |
| Background indexing indicator | HIGH | MEDIUM | P1 |
| Onboarding wizard | HIGH | MEDIUM | P1 |
| Document preview (PDF) | HIGH | MEDIUM | P1 |
| Favorites + Recent | MEDIUM | LOW | P1 |
| Auto-generated tags | MEDIUM | MEDIUM | P1 |
| Metadata filtering | MEDIUM | LOW | P1 |
| Entity extraction | HIGH | HIGH | P2 |
| Related documents | MEDIUM | MEDIUM | P2 |
| Space network graph | MEDIUM | HIGH | P2 |
| SONA self-learning | MEDIUM | HIGH | P2 |
| Manual tagging | MEDIUM | LOW | P2 |
| OCR (opt-in) | MEDIUM | HIGH | P2 |
| Spreadsheet indexing | LOW | LOW | P2 |
| Command palette | MEDIUM | MEDIUM | P2 |
| Sub-space hierarchy | MEDIUM | HIGH | P3 |
| Domain expansion | LOW | HIGH | P3 |
| Cloud embeddings (opt-in) | LOW | MEDIUM | P3 |
| Insights analytics page | LOW | MEDIUM | P3 |
| Keyboard shortcuts | LOW | LOW | P3 |
| LLM-based space naming | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for launch — validates core value proposition
- P2: Should have — add when core pipeline is stable
- P3: Nice to have — future consideration, after PMF

---

## Competitor Feature Analysis

| Feature | DEVONthink | Spotlight (macOS) | Dropbox/OneDrive | Cortex |
|---------|------------|-------------------|-------------------|--------|
| Semantic search | No (keyword + TF-IDF AI) | No (keyword) | No (keyword) | Yes — full HNSW vector search |
| Auto-organization | Suggest-and-approve (Classify) | No | No | Fully automatic GNN clustering |
| Local-first | Yes (fully local) | Yes | No (cloud sync) | Yes — no cloud required |
| Entity extraction | No | Partial (Spotlight metadata) | No | Yes — dates, amounts, people, orgs |
| Related documents | Yes (See Also — praised) | No | No | Yes — graph edges |
| Smart folder watching | Manual import or RSS | Spotlight indexes everything | Sync folder only | Watched folders with exclusions |
| File preview | Yes (many formats) | Quick Look only | Yes (web) | Yes — PDF, DOCX, images in-app |
| Privacy | High (local DB) | High (local Spotlight) | Low (cloud) | Highest — content never leaves machine by default |
| Cross-platform | macOS/iOS only | macOS only | All platforms | macOS + Windows + Linux (Tauri) |
| Self-learning | No | No | No | Yes — SONA engine |
| Graph visualization | No | No | No | Yes — space network |
| Price | $199 one-time | Free (built-in) | $10/mo | TBD |

**Key insight:** DEVONthink is the closest competitor. Its "Classify" and "See Also" features are the most-praised parts of its 20-year-old product. Cortex makes both of those automatic and adds semantic search, self-learning, and a graph layer. The positioning is: "DEVONthink if it learned from everything you did and never asked you to file anything."

**Key insight from Windows Recall backlash:** The market is starving for a local-first AI document tool that users trust. Recall's opt-in reversal and continued negative press is the market validation that Cortex's privacy-first positioning is not just a feature — it's the moat.

---

## Sources

- [DEVONthink "Is DEVONthink an AI application?" — DEVONtechnologies blog](https://www.devon.tech/blog/20250717-devonthink-ai-app) (MEDIUM confidence — vendor blog)
- [DEVONthink Honest Review: Is It Worth It in 2026? — Elephas](https://elephas.app/blog/devonthink-review) (MEDIUM confidence — independent review)
- [5 Best AI Document Management Solutions — Unite.AI](https://www.unite.ai/best-ai-document-management-solutions/) (MEDIUM confidence)
- [Best File Organization Software: AI vs Traditional Tools 2025 — TheDrive.ai](https://thedrive.ai/blog/best-file-organization-software-2025) (MEDIUM confidence)
- [Windows Copilot Goes System Wide: Privacy, Regulation, and User Backlash — Windows Forum](https://windowsforum.com/threads/windows-copilot-goes-system-wide-privacy-regulation-and-user-backlash.402990/) (HIGH confidence — documented industry event)
- [Windows 11 AI Reset: Scaling Back Copilot and Recall — Windows Forum](https://windowsforum.com/threads/windows-11-ai-reset-scaling-back-copilot-and-recall-for-privacy-and-reliability.399624/) (HIGH confidence — documented industry event)
- [Putting everything in its right place with ML-powered file organization — Dropbox Tech](https://dropbox.tech/machine-learning/smart-move-ml-ai-file-organization-automation) (MEDIUM confidence — practitioner blog)
- [MiniLM-L6-v2 embedding benchmarks — SuperMemory](https://supermemory.ai/blog/best-open-source-embedding-models-benchmarked-and-ranked/) (MEDIUM confidence)
- [5 Reasons Spotlight Isn't Good Enough (And How Fenn Fixes That) — Fenn](https://www.usefenn.com/blog/spotlight-not-good-enough-fenn-alternative) (LOW confidence — competitor marketing, useful for user pain point framing)
- [11 Critical Document Management Challenges 2025 — TheECMConsultant](https://theecmconsultant.com/document-management-challenges/) (MEDIUM confidence)
- [AI Tagging Guide: Smarter File Search — iomovo](https://www.iomovo.io/blog/the-complete-guide-to-ai-tagging-for-smarter-file-search-and-management) (LOW confidence — vendor blog, useful for pattern identification)
- DEVONthink feature analysis: Inspectors See Also & Classify documentation (HIGH confidence — official product docs)

---

*Feature research for: Cortex — Desktop document intelligence / auto-organizing document manager*
*Researched: 2026-02-27*
