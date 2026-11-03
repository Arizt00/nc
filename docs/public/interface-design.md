# Website interface

The November 2026 interface uses graphite surfaces, warm white typography, restrained copper accents and an abstract particle study. The design references light, depth and continuous forms while keeping the nC language and ncc compiler as the subject. All visible website content remains in English.

## Page structure

The homepage introduces the language, links its four levels, presents the repository's complete `hello.n` source and native compiler commands, then explains the compiler pipeline, documentation and public bootstrap scope. Language and compiler entry points, the library, downloads and the 16 documentation chapters share the navigation, theme and typography.

`tools/publishing/home.mjs` renders the homepage. `style.css` defines the shared interface. `app.js` provides search, library filtering, code copying, keyboard tab navigation, themes and the mobile menu. `scene.js` runs on all website pages and initializes the hero only where its canvas exists. `build-site.mjs` owns the common background and motion controls, selects each page's atmosphere profile, and hashes CSS and JavaScript URLs so an updated publication does not reuse an earlier interface from the browser cache.

## Motion and reading

The particle canvases are decorative; links, headings and technical explanations are ordinary HTML. The canvases have no role in compiler execution. The homepage retains a static CSS silhouette if its hero canvas is unavailable. Each page respects `prefers-reduced-motion`, has a fixed pause/play control, and suspends its frame loop when the document is hidden. The control stays hidden if the background canvas cannot initialize. Drawing scale is capped at 1.5 device pixels per CSS pixel and animation is limited to approximately 30 frames per second.

The homepage hero projects and sorts a torus by depth, with separate orbital dust, a foreground light and perspective changes driven by an eased cursor position. Nearby particles respond through repulsion and damped springs. A background canvas is fixed to the viewport on every page, including the library, downloads, language/compiler overviews and all 16 chapters. The pause state is saved under `nc-motion` and restored across navigation, browser history and same-origin tabs. Hero drawing is skipped offscreen and its particle arrays are not allocated on other pages. Smaller viewports draw fewer particles. Touch input preserves normal scrolling and does not activate mouse repulsion.

The background layer has `pointer-events: none`; text and controls remain above it. The pointer-move listener is passive. When motion is paused, cursor input does not update the camera or particle simulation. The light theme uses a darker atmospheric palette.

| Profile | Pages | Reading behavior |
| --- | --- | --- |
| Hero | Homepage | Full volume, cursor response and scroll parallax |
| Browse | Library, downloads and 404 | Fewer, dimmer particles with slower drift and gentler cursor response |
| Reading | nC/ncc overviews and 16 chapters | Lowest density, opacity and drift; no cursor glow, connecting lines or scroll parallax |

Reading pages add a stable surface behind article text and the sidebar. Code blocks and tables use solid backgrounds in both themes. Their compact pause control remains available without adding a second control. The shared renderer keeps one background canvas and one motion control per page.

Dark is the default appearance. A reader's saved light/dark preference is retained under `nc-theme` in local storage. Both themes style the library, code samples and articles. Keyboard focus remains visible, search uses a native modal dialog, and the mobile menu exposes its expanded state. The library's empty filter state stays visible while unmatched chapters are hidden.

## Validation

Run `node tools/publishing/check-public.mjs` and `node tools/publishing/check-seo.mjs` after building. The publication checker also verifies one background, one motion control, a versioned scene script and the expected profile on every non-redirect page. Browser checks cover pause/play across navigation, cursor response, coverage below the hero, Windows/Linux commands, compiler phase selection, chapter filtering, search and Escape, source copying, theme switching, and navigation between pages. Reading surfaces, code and tables were inspected in both themes; mobile pages were checked at 390 CSS pixels with no horizontal document overflow. The temporary viewport is reset after verification.

The canonical URLs, sitemap membership, page titles, descriptions and structured page identities remain owned by `seo.mjs`. This is a presentation change to the public V81 edition and does not promote a compiler milestone or change its contracts.
