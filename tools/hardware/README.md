# NVIDIA control desde nC, sin foreign

## Alcance comprobado

Implementacion de la aplicacion en un solo archivo: `nvidia_control.n`.
No contiene bloques `foreign`, fuentes C/C++, Python ni llamadas a un shell.
Usa `args`, `command`, `env`, punteros, heap y salida de la biblioteca de nC.
El proceso externo es la herramienta oficial NVIDIA instalada en
`%SystemRoot%\System32\nvidia-smi.exe`, no una implementacion simulada de NVML.

Windows x64: compilacion, objeto COFF, enlace PE interno y consultas reales
verificados con RTX 5090 Laptop GPU, controlador 610.62.
Los 20 casos y hashes estan en `evidence/nvidia_control.windows.json`.

Esto NO cierra control de voltaje, offsets de overclock, CPU/Intel/ASUS, ni
certificacion de cambios fisicos. Esos comandos devuelven 69 y no ejecutan el
proveedor. No se realizaron cambios de frecuencia, voltaje, potencia o reloj.

## Compilar

Desde la raiz del repositorio, con el Stage0 local que figura en la evidencia:

```powershell
& .\target\x86_64-pc-windows-gnu\release\ncc.exe check .\tools\hardware\nvidia_control.n
& .\target\x86_64-pc-windows-gnu\release\ncc.exe build .\tools\hardware\nvidia_control.n --out-dir .\target\hardware-nvidia-native --profile release-fast --target x86_64-pc-windows-msvc
```

No requiere los pasos FFI/LLVM/GCC de la prueba inicial. Se retiraron sus
declaraciones foreign y su manifiesto de importacion. La eliminacion de sus
artefactos temporales en `target/hardware-nvml` fue bloqueada por el entorno;
ese directorio NO forma parte de la entrega ni debe usarse para ejecutar.
La ruta normal retiene `.nm`, `.backend-object.nbo`, `.standard.mct.ll`,
`.obj`, `.entry.obj`, `.exe`, `.init` y manifiesto.
El `.nm` es el puente de enlace; no se presenta como un listado completo
instruccion por instruccion del cuerpo high-level.

## Consultas y planes

```powershell
$tool = '.\target\hardware-nvidia-native\bin\nvidia_control.exe'
& $tool self-test
& $tool probe
# Sustituir UUID por el identificador GPU-... que devuelva probe.
& $tool capabilities UUID
& $tool supported UUID
& $tool lock UUID graphics MIN_MHz MAX_MHz
& $tool defaults UUID memory
```

Los dos ultimos comandos son planes: no invocan NVIDIA ni comprueban que el
dispositivo admita esos valores. No incluyen frecuencias recomendadas.
Un UUID debe tener la forma GPU-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx.
No se admiten indices ambiguos para comandos dirigidos a un dispositivo.

La sintaxis de escritura es `lock UUID graphics|memory MIN_MHz MAX_MHz --apply`
o `defaults UUID graphics|memory --apply`. Es una solicitud explicita al
controlador, no una garantia de frecuencia efectiva, estabilidad o seguridad.
El firmware decide la admision. Se transmite el codigo de salida de NVIDIA.
No se ejecutaron esas rutas de escritura durante la validacion.

`defaults` restablece el reloj bloqueado del dominio seleccionado; no restaura
un perfil personalizado anterior ni revierte offsets, voltajes o potencia.
No hay aplicaciones automaticas al arrancar ni persistencia en el sistema.

## Ejecucion y errores

La linea se construye en un buffer manual acotado, con UUID y numeros
validados. El ejecutable se cita; no se usan CMD, PowerShell, system o popen.
Los handles estandar se pasan explicitamente para conservar la salida
redirigida sin crear un pipe que pueda bloquearse por falta de lectura.
El proceso hijo se crea sin ventana y se espera un maximo de 15 segundos.
Ante timeout se solicita su terminacion y se espera hasta 3 segundos mas.
Si estaba aplicando un ajuste, el estado del hardware se declara desconocido.
Cerrar el proceso no equivale a revertir un cambio ya aplicado al controlador.

Codigos locales: 64 argumentos invalidos, 69 proveedor/plataforma no
soportados, 70 fallo de recursos o arranque, 124 timeout/fallo de espera.
Estos codigos pueden coincidir con un codigo devuelto por NVIDIA; el mensaje
local permite distinguir el origen. El programa no inventa un exito.

## Dependencias y limites honestos

Todo el codigo nuevo de la aplicacion es nC. Eso no significa que Windows,
el driver NVIDIA o Stage0 esten escritos en nC.
La inspeccion PE muestra KERNEL32.dll, SHELL32.dll y las entradas malloc/free
de msvcrt.dll que conserva el enlace actual. No se ha afirmado eliminar el
CRT de toda la toolchain, ni se ha modificado Rust para este ejemplo.
Tampoco se ha reescrito o redistribuido nvidia-smi.

La evidencia cubre consultas fisicas, planes sin escritura, errores del
proveedor y validacion de entrada. No cubre CPU, tensiones, overclock por
offset, estabilidad bajo carga, restauracion tras bloqueo o otros hosts.

## Detalles de compatibilidad encontrados

El Stage0 probado pierde la inferencia del elemento en
`off(plan.data, ...)` cuando el puntero se obtiene de un campo: emitia stride
8 y una escritura de 64 bits. El programa conserva la semantica mediante
un alias local explicito `destination >> p<byte> <- plan.data`.
El self-test comprueba concatenacion contigua, terminador y limite del buffer.
Es una limitacion del compilador pendiente; no una correccion global de Stage0.

La ruta independiente `mct emit-llvm` tampoco incorporaba el formateador
numerico requerido por el primer prototipo. La entrega se compila por
`ncc build`, que resuelve sus modulos nC y emite los cuerpos completos.

## Referencias oficiales

- [Interfaz nvidia-smi](https://docs.nvidia.com/deploy/nvidia-smi/):
  consulta, bloqueo y restablecimiento de relojes.
- [NVML](https://docs.nvidia.com/deploy/nvml-api/api/group__nvmlDeviceQueries.html):
  los offsets de reloj son MHz, no un control generico de milivoltios.
- [Intel UVP](https://www.intel.com/content/www/us/en/support/articles/000094219/processors.html):
  restricciones de firmware y proteccion de voltaje.
