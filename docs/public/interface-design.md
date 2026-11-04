# Website interface

The November 2026 interface uses graphite surfaces, warm white typography and restrained copper accents. Its cinematic landing places an original photographic hand beneath a luminous sphere, surrounded by an orbital particle halo and a GPU-driven copper tide. The nC language and ncc compiler remain the subject. All visible website content is in English.

## Page structure

The homepage introduces the language, links its four levels, presents the repository's complete `hello.n` source and native compiler commands, then explains the compiler pipeline, documentation and public bootstrap scope. Language and compiler entry points, the library, downloads and the 16 documentation chapters share the navigation, theme and typography.

`tools/publishing/home.mjs` renders the homepage. `style.css` defines the shared interface and `landing.css` defines the approved central composition. `app.js` provides search, library filtering, code copying, keyboard tab navigation, themes and the mobile menu. `scene.js` owns the quiet background and shared motion preference on all pages. The homepage alone loads `hero-scene.js`, `hero-shaders.js`, `visual-state.js` and the vendored Three.js engine. `build-site.mjs` hashes entry assets and their local module imports to avoid reusing earlier code from the browser cache.

## Motion and reading

The particle canvases are decorative; links, headings and technical explanations are ordinary HTML. The canvases have no role in compiler execution. The homepage retains its hand photograph, CSS sphere, orbital arc and SVG tide if WebGL is unavailable. Each page respects `prefers-reduced-motion`, has a fixed pause/play control, and suspends its frame loop when hidden. Drawing scale is capped at 1.5 device pixels per CSS pixel. The hero targets 60 FPS in high/medium quality and 30 FPS in low quality; the quiet page background is capped at approximately 30 FPS.

The hero creates static GPU buffers once and moves its tidal terrain and orbital dust in GLSL. A single clock and normalized `orbPulse` synchronize sphere luminance, palm lighting and the halo's delayed light wave. High quality can add half-resolution selective bloom after a GPU timing probe. Separate landscape, tablet and mobile framing preserve the hand/sphere relationship. Cursor parallax stays small and is disabled on coarse pointers. See `cinematic-landing.md` for the rendering contract, quality budgets and measured validation.

A background canvas is fixed to the viewport on every page, including the library, downloads, language/compiler overviews and all 16 chapters. The pause state is saved under `nc-motion` and restored across navigation, browser history and same-origin tabs. Hero rendering stops offscreen; its buffers and engine are not loaded by documentation pages. Pausing finishes any remaining entrance animations and freezes the shared simulation, so resuming does not replay the entrance. Touch input preserves ordinary scrolling.

The background layer has `pointer-events: none`; text and controls remain above it. The pointer-move listener is passive. When motion is paused, cursor input does not update the camera or particle simulation. The light theme uses a darker atmospheric palette.

| Profile | Pages | Reading behavior |
| --- | --- | --- |
| Hero | Homepage | Hand, sphere, GPU halo and tidal depth, with restrained pointer parallax |
| Browse | Library, downloads and 404 | Fewer, dimmer particles with slower drift and gentler cursor response |
| Reading | nC/ncc overviews and 16 chapters | Lowest density, opacity and drift; no cursor glow, connecting lines or scroll parallax |

Reading pages add a stable surface behind article text and the sidebar. Code blocks and tables use solid backgrounds in both themes. Their compact pause control remains available without adding a second control. The shared renderer keeps one background canvas and one motion control per page.

Dark is the default appearance. A reader's saved light/dark preference is retained under `nc-theme` in local storage. Both themes style the library, code samples and articles. Keyboard focus remains visible, search uses a native modal dialog, and the mobile menu exposes its expanded state. The library's empty filter state stays visible while unmatched chapters are hidden.

## Validation

Run `node tools/publishing/check-public.mjs`, `node tools/publishing/check-seo.mjs`, `node tools/publishing/check-landing.mjs` and `node --test tools/publishing/tests/visual-state.test.mjs` after building. The publication checker verifies one background, one motion control, versioned scripts and the expected profile on every non-redirect page. Browser checks cover pause/play across navigation, compiler phases, Windows/Linux commands, filtering, search and Escape, source copying, theme switching, valid CTAs and navigation. Desktop, tablet and mobile layouts are reviewed against the approved composition. Reading surfaces remain stable in both themes. The detailed report records measured results and hardware verification limits; temporary viewport overrides are reset after verification.

The canonical URLs, sitemap membership, page titles, descriptions and structured page identities remain owned by `seo.mjs`. This is a presentation change to the public V81 edition and does not promote a compiler milestone or change its contracts.
