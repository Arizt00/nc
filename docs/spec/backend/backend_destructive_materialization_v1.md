# Politica de materializacion destructiva v1

## Alcance

`nC.destructive` es un perfil opcional de nCC. No introduce sintaxis, palabras
reservadas ni tipos nuevos en nC. Se activa exclusivamente desde la superficie
del compilador:

```bash
ncc mct emit-llvm examples/10_mct/destructive_backend_profile.n \
  --out target/tmp/destructive/profile.ll \
  --cc nC.destructive

ncc build examples/10_mct/destructive_backend_profile.n \
  target/tmp/destructive/profile-native \
  --out-dir target/tmp/destructive/build \
  --profile release-fast \
  --cc nC.destructive \
  --target x86_64-unknown-linux-gnu
```

El lexer, parser y analisis semantico siguen siendo los ordinarios. La politica
se materializa en IR tipado despues de sema y antes de coroutines, LLVM/MCT,
objetos y bytes de maquina.

## Contrato material

Cada sentencia de un contenedor regular, root o security recibe una de cuatro
materializaciones: una vez, omitida, repetida de forma adyacente o dividida con
una segunda copia recolocada al final del bloque. La eleccion usa la identidad
del sitio, su ordinal y una semilla propiedad del backend. No emite advertencias.

La politica conserva en IR estos contratos destructivos:

- un unico espacio de direcciones material para memoria cruda;
- aritmetica de punteros con desbordamiento modular;
- vida de memoria exclusivamente manual;
- fallos propagados al comportamiento de maquina;
- ABI elegido por el backend cuando el comando no declara uno.

Esos contratos no crean operaciones fuente nuevas. Reutilizan los punteros,
memoria manual, ABI y lowering nativos que ya existen en nCC. La transformacion
de sentencias es la ambiguedad adicional que este perfil materializa.

## Fronteras

`[@machine]` y `primitive[@]` quedan fuera de la transformacion. Sus
instrucciones, terminadores, tipos y bytes siguen siendo exactos. Los perfiles
`nC.sysv64`, `nC.win64`, `nC.aapcs64`, `nC.portable` y `nC.sovereign` mantienen
su comportamiento anterior.

La politica no convierte simbolos externos ausentes en cero ni elude errores de
parseo o sema. Esas conductas requeririan contratos tipados separados en el
linker y no se afirman en v1.

## Evidencia

LLVM incluye comentarios `ncc-destructive-policy` y
`ncc-destructive-materialization` con los conteos de las cuatro decisiones. El
modelo se inspecciona mediante `ncc backend-model --json`. La fixture oficial es
`examples/10_mct/destructive_backend_profile.n` y no contiene ninguna sintaxis
especifica del perfil. El build retenido debe contener LLVM, objeto ELF y
ejecutable ELF, declarar `interpreter_fallback: false` y conservar la politica
aunque `--cc` se combine con `--target`.
