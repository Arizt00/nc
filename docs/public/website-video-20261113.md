# Homepage film: integration and verification

13 November 2026.

The English homepage now includes the nC overview film, reached through
“Watch the film” in the introduction or the “See nC in 20 seconds” section.
The source explorer remains the main introduction to the language.

## Media identity

The published file is the original soundtrack edition of the Remotion film:

- 20 seconds, 1,200 frames, 60 fps;
- 1920 × 1080, H.264, Rec. 709;
- AAC stereo, 48 kHz, approximately 320 kbps;
- original instrumental music and mechanical sound effects;
- 1,517,324 bytes, with a 15,212-byte WebP poster;
- SHA-256: `02253c64e0eb4ea47eceb2120ad4abbbf76d538d88510a4edc7e2100088d58b8`.

The film demonstrates the public V81 kit. This website update does not
publish a new compiler edition or change language support claims.
Media provenance is recorded in `tools/publishing/assets/media/nc-overview.json`.

## Player behavior

The player preserves the full 16:9 picture at every width. “Play with sound”
starts preparation and playback after an explicit visitor action. Native
controls then provide pause, volume, seeking, playback options and fullscreen.
There is no automatic playback, forced mute or loop.

The current static host returns the complete MP4 with status 200 when sent a
Range request. The player therefore loads the small original file into a
local blob on activation. This gives the native player random access without
depending on server-side byte ranges. It does not re-encode the film. The
object URL is released on a non-cached page exit; a cached page retains it
for restoration.

The poster and explicit dimensions reserve the picture area. The MP4 is not
fetched by the enhancement until the visitor presses play. Without JavaScript,
the original native controls and direct download link remain available. A
preparation error restores that native route, with an English status message.
If a browser requires another play gesture, its ready player stays visible.

English WebVTT captions cover the on-screen narrative and non-speech audio.
A native disclosure contains a readable transcript, the compiler command and
links to the corresponding source examples and documentation. The download
link remains available independently of the player.

## Verification

The generator, publication references, landing checks, SEO checks and all
11 website tests pass. The tests include complete-response preparation,
network failure and rejection of HTML or empty media responses. The public
documentation workflow now runs all of those gates on Node.js 22.

The published domain serves the homepage, original MP4, poster and captions
over HTTPS with their expected MIME types. Downloaded production MP4 bytes
match the SHA-256 identity above. The revised player was exercised in the
available desktop browser: explicit start with sound, complete playback,
pause and a native seek to approximately 10 seconds passed. Captions and
the expandable transcript were also reviewed.

Responsive checks covered 320 × 844, 390 × 844, 768 × 1024, 1440 × 900 and
1920 × 1080. The picture retains 16:9 and no page-level horizontal overflow
was observed. Dark and light themes were reviewed. These are browser viewport
emulations, not physical phone tests.

The sitemap still has 21 canonical pages. The public V81 source snapshot is
preserved; only website sources, media, documentation and their inventory
records are updated in the public repository.

## Separate search observation

On the same date, a Google search for the literal domain `ncc-lenguage.org`
returned the homepage and the exact `/nc-programming-language` page, including
when Google's “Try without personalization” option was selected. Search Console
URL Inspection also confirmed that the exact language page is indexed and
served over HTTPS. This records observed indexing, not a guaranteed position
for every device, location or search session.

## Verification limits

Physical Android and iPhone devices, Safari, constrained mobile connections
and field Core Web Vitals were not tested. Playback preparation requires the
complete 1.5 MB file because this host does not provide byte-range responses.
No mobile search-ranking guarantee is inferred from the browser checks.
