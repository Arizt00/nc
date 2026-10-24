# Machine ISA para red y sincronizacion kernel v1

Estado normativo: activo.

La superficie x86 incorpora:

```text
umonitor r32|r64
umwait r32
tpause r32
serialize
```

Codificaciones:

- `UMONITOR`: `F3 [REX.W] 0F AE /6`;
- `UMWAIT`: `F2 0F AE /6`;
- `TPAUSE`: `66 0F AE /6`;
- `SERIALIZE`: `0F 01 E8`.

WAITPKG exige `features: waitpkg` y SERIALIZE exige
`features: serialize`. Encoder, decoder, feature family y roundtrip comparten
el mismo catalogo. AArch64 publica el conjunto LSE completo y RISC-V las
familias LR/SC y AMO de 32/64 bits en el coordinador kernel.

Entrada directa:

```text
type kernel_sync_waitpkg_x86_64.asm | ncc machine stdin check --syntax nasm --profile kernel --features waitpkg,serialize
cat kernel_sync_waitpkg_x86_64.asm | ncc machine stdin emit-bytes --syntax nasm --profile kernel --features waitpkg,serialize --out sync.bin
```

`--features` se incorpora al contrato `.nm` antes de sema. Omitir un gate
produce `E_MACHINE_FEATURE_REQUIRED_001`; no se emiten bytes silenciosamente.

El ejemplo canónico genera 20 bytes:

```text
0f ae f0 f3 48 0f ae f0 f2 0f ae f0 66 0f ae f0 0f 01 e8 c3
```
