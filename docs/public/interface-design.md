# Website interface

The November 2026 interface puts the language's source, architecture and
published evidence first. A programmer can inspect real nC, find a native kit
and follow the compiler's contracts from the homepage. The hand and luminous
sphere remain a secondary project symbol. All public website text is English.

The maintained visual contract is [DESIGN.md](../../tools/publishing/DESIGN.md).
[Website positioning](website-positioning.md) records the audience, research,
skill sources and the distinction between design objectives and verified scope.

## Page structure

The first screen pairs a concise introduction with a four-tab source explorer:
ordinary functions, root `[@]`, `[@machine]` and `primitive[@]`. Source comes
from the public manual and repository fixtures. Each example links to its
original file and a contract explanation. The machine example identifies its
explicit overflow branch; the primitive example identifies privileged operations
and their execution requirements. These are source views, not a browser compiler.

The following sections explain the design choices for programmers coming from
C, the compiler pipeline, a complete first program and the V81 evidence. The
five compiler phases are separate from the four language surfaces. The page
then presents the animated symbol and routes to the two manuals and source.

The library has four practical entry points: understand the design, install
the toolchain, build a first program and follow the architecture. Its 16
chapters, language/compiler overviews, downloads and 404 page share the same
navigation, typography and themes. Existing canonical URLs and redirects remain.

## Visual system

IBM Plex Sans carries headings and interface text; IBM Plex Mono carries source
and commands. Original WOFF2 files are served from the website, with their SIL
Open Font License, source URLs and SHA-256 identities. The first screen does
not depend on a remote font service.

Graphite surfaces, warm white text and restrained copper distinguish code and
active controls. The light theme has stable white reading surfaces. Body text
uses comfortable line lengths, while code and tables scroll inside their own
containers. Mobile source text remains at least 12 px. Heading hierarchy and
structural layouts follow the content rather than a repeated promotional card.

## Interaction

Source tabs expose `aria-selected`, controlled panels and roving keyboard
focus. Arrows, Home and End switch examples. `?surface=machine`, for example,
opens the machine source directly; an invalid value falls back to Functions.
The copy action uses the active panel's text, preserving its code and spacing.
The generated `surfaces.json` retains each example's original SHA-256 identity.
Without JavaScript, the default source remains visible and the other original
files are linked.

Search uses a native modal dialog with a labelled input. The library filter
reports matches and its empty state, and stores its term in `?chapter=`.
Reading progress, return-to-chapter links, source copying, Windows/Linux command
tabs, compiler phase tabs and the mobile navigation remain functional. The
selected phase and operating system are shareable through `?phase=` and `?os=`. Escape
closes the dialog or mobile menu. Both themes retain visible keyboard focus.

## Motion and progressive loading

The first screen presents its text and source immediately. `symbol-loader.js`
uses an intersection observer to import the GPU scene near the secondary
section. The hand image loads lazily. A paused or reduced-motion visit keeps
the layered static symbol until motion is explicitly enabled. The loader and
renderer handle browser-history restoration.

The symbol uses one Three.js engine, static point buffers and GLSL movement.
Its shared clock and normalized `orbPulse` synchronize sphere brightness,
palm lighting and the halo's delayed light wave. Tidal depth, bounded pointer
parallax, three quality profiles and selective high-quality bloom are retained.
Scene coordinates belong to the figure, so a responsive text column cannot
displace the sphere relative to the hand.

The GPU loop stops off screen, when the document is hidden or when motion is
paused. A CSS sphere, orbital arc and SVG tide remain if WebGL is unavailable.
`?visual=fallback` selects that path for verification. GPU buffers and resources
are released when leaving a non-cached document.

The quiet page background continues across the website. Reading pages have
the lowest density and drift, no cursor glow or scroll parallax, and stable
surfaces behind prose, code and tables. One persistent motion control governs
both renderers; its saved `nc-motion` preference survives navigation. Touch
scrolling does not drive the pointer effect.

## Implementation and checks

| Files | Responsibility |
| --- | --- |
| `home.mjs`, `surfaces.mjs` | Homepage structure, real examples, safe highlighting and source provenance |
| `style.css`, `landing.css` | Shared tokens/layout and the secondary layered symbol |
| `app.js` | Search, source tabs, URL state, copying, filtering, themes and navigation |
| `symbol-loader.js` | Deferred GPU loading and history restoration |
| `hero-scene.js`, `hero-shaders.js`, `visual-state.js` | Figure coordinates, rendering, shared pulse and quality |
| `scene.js` | Quiet backgrounds and the shared motion preference |
| `build-site.mjs`, `seo.mjs` | Static output, dependency-aware fingerprints and canonical identities |

The build hashes emitted bytes after recursively versioning local module
dependencies. A shader change therefore also changes the URLs of its importers.
The landing checker verifies dependency hashes, original-source hashes,
accessible control targets, local engine/font assets and the image budget.

Reproduce the website checks from the curated public repository:

```sh
node tools/publishing/build-site.mjs
node tools/publishing/check-public.mjs
node tools/publishing/check-seo.mjs
node tools/publishing/check-landing.mjs
node --test tools/publishing/tests/surfaces.test.mjs tools/publishing/tests/visual-state.test.mjs
```

These are website checks. They do not rebuild the compiler or certify its
retained tests. [The interface validation report](website-validation-20261104.md)
records the browser checks, measurements and remaining device limits.
