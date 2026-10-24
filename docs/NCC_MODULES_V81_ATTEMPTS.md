# V81 attempts and controls

English edition of the retained attempt log. Failures and final runs remain distinct.

## Baseline

V80 C7 rejects the overaligned program on both hosts while materializing the first global. The error is retained, and absence of the requested object is verified.

## Fixture preparation

The first unwind-fixture preflight used the reserved word local as an identifier. It was changed to frameValue in the fixture; the parser was not relaxed.

Leaf-child functions initially appeared under stc rather than fn/public. The resolver rejected their three inaccessible references. The fixture was corrected without changing visibility rules.

A cache-status query used an incorrect report path. The cold build had already succeeded; its original `.ncc/incremental/project-cache.json` report was retained before the warm pass.

## Preliminary reconstructions

Windows B1 lost its process after 70 of 74 objects, without an image or completion receipt. The cause is not certified, and the attempt is not counted as a clean build. A provisional seed linked those objects with four V80 nC objects whose sources were verified identical and a retained nC entry. Its receipt explicitly declares retained inputs. It is neither B2 nor C2.

The provisional A2 attempt was cancelled once that seed allowed B2 to start in a completely new directory. Its materials and cancellation record were retained. Final B2/C2 do not reuse its objects.

Linux B1 completed as a preliminary seed. B2/C2 use the final snapshot with V81 cache identity and MCT validation.

## Linkers

MSVC rejected 8192-aligned COFF with LNK1164 when `/ALIGN:8192` was absent. Explicit linking with the option produced a runnable image. LNK4108 is retained, not suppressed or presented as driver certification.

COFF rejects global fixtures aligned above 8192 bytes, while their IR/MCT and direct PE executables are retained. This is an object-format limit, not accepted truncation.

## Runtime and corpus

The first Linux B2 run of 38 contracts lacked the names, empty and stress directories. Three filesystem contracts failed. The report and fixtures were retained; the retry uses fixtures-r2 and processes-r2.json with complete preparation. Windows B2/C2 and Linux C2 received that preparation before their first runs and passed all 38 contracts.

The initial Windows B2 corpus lacked ncruntime.dll beside generated programs. The loader rejected 55 provider-dependent cases with 0xc0000135. Generated code was not changed to resolve this. That run is retained; B2/C2 were repeated in fresh windows-r2 directories with the identified native provider. All four final corpus runs pass.

## Intentional negative controls

- align(24) and align(2097152) produce no valid image or object.
- COFF does not truncate globals aligned above 8192 bytes.
- Changing one data byte causes golden verification to fail.
- A non-power-of-two MCT header, insufficient declared alignment and a misaligned global offset are rejected despite matching cache digests.
- Equivalence verification identifies the altered byte at offset 31 exactly. That image is not executed.

FAIL results from intentional controls and retained attempts remain separate from final positive reports. They are not deleted to obtain a passing suite.
