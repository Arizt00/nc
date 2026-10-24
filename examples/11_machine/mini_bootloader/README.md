# Mini bootloader nC

Este prototipo mantiene separadas las cinco autoridades materiales:

- `main.n`: coordinador system y madre del contrato de construccion.
- `src/mini_boot_kernel.n`: cuerpo kernel manual y freestanding.
- `mini_boot_entry.nm`: entrada machine x86_64 con relocacion `rel32` a `kmain`.
- `mini_boot_root.nb`: raiz Basal editable y verificable.
- `mini_boot_board.ncp`: contrato Primitive de reset, puertos y entrada.

Validacion y materializacion:

```powershell
ncc project sync .
ncc project check .
nbuild main.n --target x86_64-pc-windows-msvc --profile release-fast
ncc machine emit-object mini_boot_entry.nm --out .ncc/obj/mini_boot_entry.o
ncc basal check mini_boot_root.nb
ncc primitive check mini_boot_board.ncp
```

El ejecutable hosted demuestra el contrato de coordinacion. `system` y `kernel`
no se invocan como funciones entre si: la continuidad material se declara en
Basal/Primitive y la relocacion de `mini_boot_entry.nm` enlaza `kmain`. La entrada
machine contiene instrucciones privilegiadas y no debe ejecutarse como proceso
de usuario.
