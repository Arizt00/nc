# Website interface validation — 4 November 2026

## Scope and result

The source-first interface was reviewed against the newer direction: code,
architecture and technical evidence lead; the hand and sphere are secondary.
The implementation retains English content, real routes and the existing
public V81 release scope. The earlier cinematic capture remains a composition
reference for the symbol, not the homepage hierarchy.

## Automated verification

| Check | Result |
| --- | --- |
| Static generation | 21 canonical pages, 16 documentation chapters |
| Publication references | 40 HTML files including legacy redirects; 667 local references resolve |
| Search and reading profiles | 18 searchable entries; 22 non-redirect backgrounds; 18 reading profiles |
| SEO identities | 21 unique titles, descriptions, canonical URLs and structured page identities |
| Landing structure | 53 unique IDs; valid anchors and interactive targets |
| Source provenance | Four examples tied to public source SHA-256 identities |
| Local graphics graph | Four checked modules; import fingerprints match emitted dependency bytes |
| Production image | Original hand WebP: 78,040 bytes, below the 250 KB budget |
| Source and visual-state tests | 8 passed, 0 failed |

The source tests verify reversible highlighting and HTML escaping, unchanged
source provenance, real reference links and accessible panel relationships.
Visual-state tests verify pulse continuity, clock freezing/resumption and bounded
quality reduction. The first cache-graph implementation exposed an ordering
error; recursive dependency fingerprinting corrected it before publication.

## Browser review

The local build was exercised in the available desktop in-app browser. Viewport
sizes are emulations on this machine, not physical phone or tablet results.

| Criterion | Result |
| --- | --- |
| First screen and full-page rhythm at 1440 × 900 and 1920 × 1080 | Passed after code-size and header refinements |
| Mobile at 390 × 844; tablet at 768 × 1024 | Passed |
| Narrow reflow at 320 px | No page-level horizontal overflow; code retains its own scroll container |
| Source tabs and URL restoration | Passed for Functions, Root, Machine and Primitive; one selected panel |
| Keyboard tabs | Arrows, Home and End select the corresponding source tab |
| Source and command copy controls | Browser reports successful clipboard writes with “Code copied” feedback |
| Compiler phases | Materialization changes the contract text, reference and link; `?phase=3` restores it |
| Windows/Linux commands | Correct shell, target and library-path recipe selected; `?os=linux` restores it |
| Library filter | Empty state, matching chapters and query restoration passed |
| Documentation search | ABI returns 11 matches; Escape closes the dialog |
| Mobile menu | Navigation opens; Escape closes; documentation route works |
| Light/dark reading | First-program chapter reviewed in both themes; stable code surface |
| Pause and navigation persistence | Shared time remains fixed while paused; state survives navigation |
| Deferred GPU loading | First screen reports `waiting`; scene becomes WebGL2 near the symbol |
| Static fallback | `?visual=fallback` retains the layered hand/sphere and working content |
| Browser console | No errors or warnings observed during the reviewed flow |

The copy result verifies the browser application's successful write and
feedback; it does not certify paste behavior in every operating-system editor.
All production source examples are inspected and hashed; this website task
does not constitute a new compiler execution certification for those fixtures.

## Observed scene measurements

The secondary scene's own diagnostics reported the following local samples:

| Viewport | Quality | Points | Frame-rate sample | GPU sample | CPU submission |
| --- | --- | --- | --- | --- | --- |
| 1440 × 900 | High, selective bloom | 53,000 | 56.1 FPS | 0.98 ms | 0.30 ms |
| 390 × 844 | Low, no bloom | 17,000 | 30.0 FPS | 2.97 ms | 0.10 ms |

The desktop observation recorded CLS `0.0000`, LCP 72 ms and a maximum observed
event duration of 24 ms. The mobile observation recorded LCP 60 ms. These were
warm local navigations on the available machine. They are not cold-network
results, field Core Web Vitals, a measured INP percentile or phone GPU benchmarks.
The quality policy may choose a different tier on another device.

## Remaining verification limits

Physical low-end phones, iOS/Safari, OS-level reduced-motion preference changes,
forced GPU context loss, battery impact and throttled-network Lighthouse were
not tested here. The static fallback and manual persistent pause were tested;
the corresponding media-query and context-restoration handlers remain in code.

The public release remains V81. This publication changes presentation and
navigation; it does not promote Stage3, replace a seed, publish private V117
compiler work or declare the compiler's historical suites green.
