# Website visual assets

## hand-hero-v1.webp

Original hand-and-forearm image generated for this project on 4 November 2026
with the built-in image generation tool. The supplied landing was used as a
composition and lighting reference. The production image contains only the
hand and forearm, with transparency; it does not contain the sphere, website
text, navigation, particle halo, or tide.

The original generated PNG is retained in the local publication evidence.
The 1536 × 1024 WebP is a format conversion, preserving alpha, produced with
FFmpeg's libwebp encoder at quality 86. Its source prompt is recorded in
`hand-hero-v1-prompt.txt`. Reference photographs and the approved page capture
are not distributed as production website assets.

## Geometry and interface graphics

The sphere, halo, orbital line and tidal field are rendered from project
geometry and GLSL sources. The small interface symbols and static tidal
fallback are original SVG paths in `home.mjs`. Editorial type uses the user's
installed Times New Roman or Georgia serif fonts, with no font downloads.

## Graphics engine

Three.js 0.187.0 is bundled locally in `../vendor/three/0.187.0/` with its MIT
license. Source: https://github.com/mrdoob/three.js/tree/r187
Package: https://registry.npmjs.org/three/-/three-0.187.0.tgz

The engine was bundled from its published `three.module.js` and `three.core.js`
using esbuild 0.25.12 with `--bundle --format=esm --minify
--legal-comments=inline`. No third-party code is fetched at page runtime.
