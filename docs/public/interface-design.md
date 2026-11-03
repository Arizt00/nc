# Website interface

The November 2026 interface uses graphite surfaces, warm white typography, restrained copper accents and an abstract particle study. The design references light, depth and continuous forms while keeping the nC language and ncc compiler as the subject. All visible website content remains in English.

## Page structure

The homepage introduces the language, links its four levels, presents the repository's complete `hello.n` source and native compiler commands, then explains the compiler pipeline, documentation and public bootstrap scope. Language and compiler entry points, the library, downloads and the 16 documentation chapters share the navigation, theme and typography.

`tools/publishing/home.mjs` renders the homepage. `style.css` defines the shared interface. `app.js` provides search, library filtering, code copying, keyboard tab navigation, themes and the mobile menu. `scene.js` runs only on the homepage. `build-site.mjs` hashes CSS and JavaScript URLs so an updated publication does not reuse an earlier interface from the browser cache.

## Motion and reading

The particle canvases are decorative; links, headings and technical explanations are ordinary HTML. The canvases have no role in compiler execution. A static CSS silhouette remains available if JavaScript or canvas is unavailable. The scene respects `prefers-reduced-motion`, has a fixed pause/play control available throughout the homepage, and suspends its shared frame loop when the document is hidden. The control stays hidden if canvas initialization cannot complete. Drawing scale is capped at 1.5 device pixels per CSS pixel and animation is limited to approximately 30 frames per second.

The hero projects and sorts a torus by depth, with separate orbital dust, a foreground light and perspective changes driven by an eased cursor position. Nearby particles respond through repulsion and damped springs. A second canvas is fixed to the viewport and covers the entire homepage, including the sections below the hero and the footer. Its depth layers react to the cursor and use gentle scroll parallax. Both canvases share the pause state, saved under `nc-motion`. Hero drawing is skipped offscreen while the ambient field continues. Smaller viewports draw fewer particles. Touch input preserves normal scrolling and does not activate mouse repulsion.

The background layer has `pointer-events: none`; text and controls remain above it. Cursor listeners are passive. When motion is paused, cursor input does not update the camera or particle simulation. The light theme uses a darker atmospheric palette. The effect is limited to the homepage so technical reading pages retain their existing interface.

Dark is the default appearance. A reader's saved light/dark preference is retained under `nc-theme` in local storage. Both themes style the library, code samples and articles. Keyboard focus remains visible, search uses a native modal dialog, and the mobile menu exposes its expanded state. The library's empty filter state stays visible while unmatched chapters are hidden.

## Validation

Run `node tools/publishing/check-public.mjs` and `node tools/publishing/check-seo.mjs` after building. Browser checks cover pause/play, cursor response, coverage below the hero, Windows/Linux commands, compiler phase selection, chapter filtering, search and Escape, source copying, theme switching, and navigation between pages. The mobile menu, article layout, downloads and homepage were inspected at 390 CSS pixels, with no horizontal document overflow. The temporary viewport is reset after verification.

The canonical URLs, sitemap membership, page titles, descriptions and structured page identities remain owned by `seo.mjs`. This is a presentation change to the public V81 edition and does not promote a compiler milestone or change its contracts.
