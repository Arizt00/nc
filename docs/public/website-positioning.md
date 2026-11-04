# nC website: product argument and design research

4 November 2026. This document records the reasoning for the systems-language
website direction. The public technical scope remains bootstrap V81.

## What nC needs to show

The intended visitor should understand three things quickly: nC is a systems
language, its distinctive idea is continuity across language levels, and there
is source, documentation and a native bootstrap kit to inspect. Visual polish
supports those facts. The main experience is evaluating and learning a language.

The first screen therefore includes real source, a concise proposition and
direct paths to the guide and native kit. A surface explorer connects ordinary
source, root contracts, typed machine instructions and material primitives to
their corresponding references. Compiler phases are explained separately.
Release evidence and limitations remain adjacent to the installation path.

## The relationship to C

The project's goal is C-class control with a richer continuous model. Its
website should explain the design choices that pursue that goal. A blanket
claim of superiority would hide the very contracts a systems programmer needs
to assess. Performance parity or superiority requires published benchmarks;
ecosystem maturity and complete feature parity are not established by a source
example or a compiler reconstruction.

| Area | Grounded statement | Website evidence |
| --- | --- | --- |
| Machine boundary | GCC extended asm expresses instructions through a template plus operand/clobber constraints. nC models typed instructions and control flow in its machine layer. | GCC reference; nC machine guide and checked-overflow fixture |
| Representation | Layout, alignment, ABI and linking affect observable behavior. nC treats them as part of the continuity contract. | V81 ABI/layout record and materialization guide |
| Memory | nC retains manual/raw responsibility. Typed representation does not establish ownership, bounds or pointer-lifetime safety. | nC syntax and memory documentation |
| Bootstrap | Public B/C identity applies to named generations within each tested host. It is reproducibility evidence, not a cross-language speed result. | V81 dossier and reproduction recipe |

The GCC comparison describes an interface boundary; it does not imply that
C compilers cannot analyze constraints or that all inline assembly is unchecked.
Clang's sanitizer documentation also shows why a claim that nC has solved C's
undefined-behavior or memory-safety problems would be unsupported by the current
manual/raw contract.

## Design research and skills

- [Frontend Design](https://github.com/anthropics/skills/tree/ca45fc496a3436251f191354091d557da31c15b5/skills/frontend-design): ground visual identity in the subject, use real content and critique generic defaults. Applied to source-first composition and the nC surface explorer.
- [Web Interface Guidelines](https://github.com/vercel-labs/agent-skills/tree/1a2c79d4e6c4a2283dc2c614b6538cbe1ed5ca62/skills/web-design-guidelines): applied to semantics, focus, copy controls, reduced motion, fonts, responsive code panels and navigation state.
- [Impeccable typography reference](https://github.com/pbakaus/impeccable/blob/5af4319c4e7a7ee3e5e2b9fe899bcfb21c6190cb/plugin/skills/impeccable/reference/typeset.md): studied the separation between expressive presentation and stable reading surfaces. Its binary tooling is not required by this website.
- The nine project `nc-*` skills continue to govern the symbol, shader work, performance, frontend structure and visual verification. Their visual priorities are interpreted with the user's newer direction: the symbol is secondary.

The two newly installed skills are instruction packages, not a replacement
for research, implementation or browser verification. Their pinned source
revisions make the learning inputs inspectable.

## Primary-site observations

These are design observations, not borrowed product claims or copied layouts:

- [Zig](https://ziglang.org/) places a working code sample and its command beside concrete language/toolchain capabilities. nC should make source and a first command easy to find.
- [Rust](https://rust-lang.org/) gives clear routes to installation, learning and tools. nC needs similarly distinct entry points while reporting its own maturity and guarantees.
- [Odin](https://odin-lang.org/) connects the language with real software and examples. nC currently has compiler, bootstrap and fixture evidence; that is the material it can honestly show rather than invented users or projects.

## Maintained design and source references

The token system, layout and interaction contract are in
`tools/publishing/DESIGN.md`. The public language sources remain
`docs/syntax.md`, `docs/machine.md`, `docs/primitive.md`, `docs/ffi.md`, the two
public manuals and the V81 dossier. The website does not modify their semantics.

External technical references:

- [GCC Extended Asm](https://gcc.gnu.org/onlinedocs/gcc/Extended-Asm.html).
- [Clang UndefinedBehaviorSanitizer](https://clang.llvm.org/docs/UndefinedBehaviorSanitizer.html).
- [IBM Plex](https://github.com/IBM/plex), with original font files and their license retained in the website assets.

## Release gate

Before publication: review the rendered design, test source tabs and URL state,
copy exact source, verify all routes and anchors, inspect mobile and reading
surfaces, confirm reduced-motion/fallback behavior, run publication/SEO checks,
and verify the curated publication manifest. Report device and measurement
limits. Keep the existing custom domain and public audience.
