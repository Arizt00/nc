# nC website implementation

This directory builds the English public website. Compiler work has separate
contracts in the repository root; website tasks must not change the compiler,
runtime, bootstrap seed, published compiler evidence, or support claims.

For the cinematic landing, use `docs/public/cinematic-landing.md` as the
maintained implementation contract. The local design brief is
`MASTER_PROMPT_CODEX.md`, supported by the nine `nc-*` project skills and
`references/approved-landing.png`. Those supplied design materials may be
absent from the curated public export. Reference
photographs do not establish redistribution rights. Production assets must
be original or have an identified license; never publish a screenshot with
fake controls placed over it.

Keep text, navigation and controls as accessible HTML. The hand, sphere,
orbital dust and copper tide remain separate layers. One clock and normalized
`orbPulse` drive the sphere, palm lighting and halo. Particle motion belongs
in GPU shaders, with static geometry buffers and a single graphics engine.
Use a quiet particle profile on reading pages.

Retain functional routes, search, code copying, compiler tabs and release
links. Respect reduced motion and persistent pause. Provide a usable static
fallback if WebGL fails. Bound DPR and particle work; stop rendering when
hidden or off screen and release graphics resources when leaving the page.

Before publication, run the generator, publication checks, SEO checks and
visual-state tests. Review desktop, mobile and tablet captures against the
approved reference, correct observed differences, and document real metrics
and remaining verification limits. Do not claim a frame rate or field Core
Web Vitals result without the corresponding evidence.
