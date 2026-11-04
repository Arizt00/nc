# nC cinematic landing — implementation and validation

Date: 4 November 2026. Website: https://ncc-lenguage.org/.

The landing follows the supplied approved composition: a centered hand and
white sphere, a broad orbital dust halo, the serif headline “One language.
Every layer.”, functional navigation, two primary actions, a three-part
feature strip and a lower copper particle tide. The screenshot is a design
reference, not a production background. The compiler and its published V81
support scope are unchanged.

## Design inputs and applied skills

The supplied `MASTER_PROMPT_CODEX.md` and all nine project skills were read.
They are retained in the local workspace with the supplied references.

| Skill | Applied responsibility |
| --- | --- |
| nc-art-direction | Central composition, graphite/ivory/copper palette, serif hierarchy, separate portrait framing |
| nc-frontend-engineering | Existing static generator, semantic HTML, live routes and existing product interactions |
| nc-photorealistic-hero | Original transparent hand asset, separate sphere, palm illumination and light shaft |
| nc-shader-engineering | One Three.js renderer, GLSL materials, static buffers, bounded DPR and cleanup |
| nc-tidal-particles | Perspective terrain, travelling waves, jittered distribution, depth and apparent defocus |
| nc-reactive-halo | Irregular orbital dust and a delayed light wave driven by the shared pulse |
| nc-motion-choreography | Short overlapping entrances, restrained pointer parallax, pause without replay |
| nc-performance-accessibility | Three quality tiers, GPU timing, selective bloom gate, reduced motion and static fallback |
| nc-visual-qa | Four viewport sizes, motion captures, corrective passes, functional tests and recorded limits |

The approved capture and third-party inspiration photographs are not included
in the deployed assets. The hand was generated as an original project asset
with the built-in image generation tool. Its prompt and provenance are in
`tools/publishing/assets/`. The pose follows the approved composition, but
the original hand is not pixel-identical to the reference photograph.

## Layer and rendering contract

1. A dark tonal background establishes atmosphere without moving content.
2. A transparent WebGL canvas draws the halo, sphere and tidal field.
3. The hand photograph, masked relighting image, warm palm overlay and soft
   light shaft are separate DOM layers.
4. The headline, subtitle, navigation, CTAs and feature links are actual HTML
   above the scene. They remain selectable and available without WebGL.
5. The existing language levels, source example, native command tabs, compiler
   pipeline, documentation, project scope and contribution content follow.

The viable static stack is retained. No React/Next migration or application
server is required. Only the homepage loads the graphics engine. Three.js
0.187.0 is vendored with its MIT license; no runtime CDN is needed. The hand
is a 1536 × 1024 WebP with alpha, 78,040 bytes. The engine bundle is 748,850
bytes uncompressed and 191,733 bytes with local gzip; actual HTTP transfer
encoding is a hosting concern.

The shared state holds `time`, `orbPulse`, `orbWorldPosition`, `qualityTier`
and `paused`. The normalized pulse is:

```text
orbPulse(t) = 0.5 + 0.5 × sin(0.82 × t)
```

The sphere shader and DOM relighting use the same current pulse. Halo dust
mixes that signal with the same oscillator evaluated at a distance-dependent
delay. The clock stops while paused, hidden or outside the viewport. Return
from visibility changes resets the timestamp baseline rather than catching
up hidden time. Repeated navigation releases renderer resources on actual
page departure and preserves the browser-history case.

Halo and tide attributes are seeded once. Their vertex shaders receive time
and a small set of uniforms; JavaScript does not rewrite thousands of point
positions per frame. Quality reduction changes the draw range of shuffled
buffers, preserving coverage instead of cutting away a band of terrain.

The tide uses a jittered grid, a perspective camera, intersecting travelling
waves and depth-dependent point size/alpha. Near points soften into restrained
bokeh. `tideParameters` in `visual-state.js` exposes amplitude, speed,
wavelength, particle size, opacity and copper tint.

## Quality and light budgets

| Tier | Halo points | Tide points | DPR cap | Target cadence | Postprocessing |
| --- | ---: | ---: | ---: | ---: | --- |
| High | 19,000 | 34,000 | 1.5 | 60 FPS | Selective bloom after GPU probe |
| Medium | 12,000 | 22,000 | 1.25 | 60 FPS | Analytic sphere glow |
| Low | 7,500 | 9,500 | 1.0 | 30 FPS | Analytic sphere glow |

The initial tier considers viewport, cores, reported memory and data saving.
Two slow measurement windows reduce quality. The normal page adapts; an
explicit `?quality=high`, `medium` or `low` fixes a profile for comparison.

High quality activates bloom only after a GPU timer result below 8 ms. Its
half-resolution light target includes the sphere/halo scene, not text or the
photograph. A brightness threshold and two separable blur directions isolate
the light. A subsequent result above 12 ms disables this pass. Baseline
rendering uses six draw calls; the selective bloom path uses thirteen.

## Access and degradation

- The shared Pause/Play control persists under `nc-motion` across routes,
  history and same-origin tabs. It finishes entrance animations when pausing,
  then freezes both simulation time and light. Resuming preserves phase.
- `prefers-reduced-motion` suppresses long entrances and selects a still
  scene by default. An explicit user Play choice is respected.
- Cursor parallax is a few pixels and disabled for coarse pointers. Touch
  input retains ordinary scrolling.
- A transparent hand, CSS sphere, orbital arc and static SVG tide remain if
  WebGL2 fails. `?visual=fallback` exposes this path for inspection.
- The skip link, native search dialog, focus indicators, expanded mobile menu,
  Escape behavior and keyboard-operated tab lists remain available.
- Library, downloads and documentation retain their global quiet atmosphere.
  Reading pages use the lowest density and stable article surfaces; code and
  tables have solid backgrounds in dark and light themes.

## Verification

Commands from the curated public repository root:

```text
node tools/publishing/build-site.mjs
node tools/publishing/check-public.mjs
node tools/publishing/check-seo.mjs
node tools/publishing/check-landing.mjs
node --test tools/publishing/tests/visual-state.test.mjs
```

For local preview, serve `sites/nc-language/dist` with an HTTP server that maps
extensionless routes to their `.html` files. The local validation server used
`http://127.0.0.1:4327/`. No npm installation or compiler rebuild is required
to generate this static site from the checked-in files.

Checks passed: 40 HTML files, 619 local references, 18 search entries, 22 shared
atmosphere/control profiles, 21 canonical sitemap pages, 21 unique titles and
descriptions, 21 structured-data graphs, 37 unique landing IDs and all local
graphics imports. The five unit tests cover pulse bounds/periodicity,
pause/resume phase, monotonic time, initial quality selection and decreasing
quality budgets.

Browser checks passed for the language/compiler routes, Get Started,
architecture anchor, compiler phase selection, Windows/Linux command tabs,
source copying, search and Escape, library filtering, download destinations,
dark/light reading surfaces, menu expansion and Escape focus, persistent
pause, and the forced static fallback. No shader or JavaScript warnings or
errors were reported in the checked final runs.

## Recorded browser measurements

These are local, warm-cache viewport simulations in the available desktop
browser. They are not measurements on four physical devices, mobile network
tests, field Core Web Vitals, or a universal frame-rate guarantee.

| CSS viewport | Profile | Particles | Observed FPS | GPU sample | LCP sample | CLS |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1920 × 1080 | High, selective bloom | 53,000 | 60.0 | 5.54 ms | 132 ms | 0 |
| 1440 × 900 | High, selective bloom | 53,000 | 60.0 | 5.72 ms | 132 ms | 0 |
| 768 × 1024 | Medium | 34,000 | 60.0 | 0.83 ms | 380 ms | 0 |
| 390 × 844 | Low | 17,000 | 30.0 | 0.61 ms | 92 ms | 0 |

CPU submission samples were about 0.2–0.6 ms. The final mobile menu/pause
exercise recorded a maximum Event Timing duration of 40 ms; this is not a
field INP measurement. An earlier run during preview/viewport transitions
recorded approximately 1 FPS and a 3,768 ms event duration. Its low callback
cadence was not isolated to a definitive cause; it is retained as an initial
observation, not relabeled as a passing performance benchmark.

## Visual review and corrections

| Criterion | Internal review result |
| --- | --- |
| Centered hand/sphere composition and serif hierarchy | Passed; original hand differs from the supplied photograph |
| Organic lower tide and depth | Passed; static buffers, GPU waves and changing captured crests |
| Shared sphere/hand/halo pulse | Passed; same clock/uniform contract, changing luminance, verified frozen phase on pause |
| Functional HTML and existing content | Passed |
| Desktop/tablet/mobile layout without horizontal overflow | Passed at all four recorded sizes |
| WebGL-unavailable path and manual pause | Passed |
| Physical low-end phone performance and OS-level motion preference | Requires device validation |
| Actual GPU context loss/restoration on device | Not checked; handlers implemented |
| Field INP/LCP and throttled-network Lighthouse audit | Not checked |

Corrective passes softened the sphere edge and glow, broke up the uniform
halo, lowered the tide behind the CTAs, brought the orbital marker inside the
viewport, gave portrait tablets a larger sphere and higher halo, separated
mobile CTAs from the advance control, enlarged mobile feature captions and
stopped pause/resume from replaying the entrance.

Local screenshots and machine-readable evidence are retained under
`_tmp/nc-publication/`: `landing-final-1920-20261104.png`,
`landing-final-1440-20261104.png`, `landing-final-768-20261104.png`,
`landing-final-390-20261104.png`, `landing-fallback-20261104.png`,
`landing-browser-validation-20261104.json` and the animation sequence files.
Capture labels 0/3/8 describe the requested sampling sequence; automation
overhead extended the actual recorded scene timestamps to approximately
0.383, 14.292 and 33.791 seconds. The JSON preserves those actual values.

## Source files

| Files | Responsibility |
| --- | --- |
| `tools/publishing/home.mjs`, `build-site.mjs` | Landing DOM, common header selection, static build and versioned local modules |
| `tools/publishing/landing.css` | Approved composition, responsive framing, layered light and entrances |
| `tools/publishing/hero-scene.js`, `hero-shaders.js`, `visual-state.js` | Renderer, GPU fields, selective bloom, shared pulse and quality policy |
| `tools/publishing/scene.js` | Quiet page background and shared motion preference |
| `tools/publishing/assets/`, `vendor/three/0.187.0/` | Original hand asset, prompt/provenance and licensed local engine |
| `tools/publishing/check-landing.mjs`, `tests/visual-state.test.mjs` | Structural checks and animation/quality contract tests |
| `sites/nc-language/dist/` | Generated production pages and local assets |

This publication changes the website interface. It does not promote Stage3,
replace a native seed, rebuild the compiler, or certify retained compiler tests.
