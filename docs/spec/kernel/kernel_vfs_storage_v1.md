# Kernel VFS and writable storage v2

Estado normativo: activo. Sustituye el perfil inicial read-only del 27 de
julio de 2026.

## VFS

El VFS normaliza rutas absolutas, resuelve enlaces, conserva handles durante
`rename`, admite sparse writes y rechaza ciclos y desmontajes ocupados. Cada
nodo materializa `mode`, `uid`, `gid`, tamano y generacion.

`open_as` aplica permisos POSIX owner/group/other y ACL de usuario, grupo,
mascara y other. UID 0 conserva bypass explicito. `chmod`, `chown`, `set_acl`,
snapshot de arbol y borrado de mount son operaciones visibles.

`KernelSystem.vfs` es un `MountedVfs`; no existe una segunda jerarquia oculta.

## FAT escribible

FAT12, FAT16 y FAT32 se materializan de nuevo desde el snapshot VFS:

- cadenas de cluster y entradas FAT espejadas;
- directorio raiz FAT12/16 y cluster raiz FAT32;
- FSInfo FAT32;
- nombres 8.3 unicos;
- LFN UTF-16 con ordinales y checksum;
- directorios recursivos, archivos vacios y datos multi-cluster.

El lector reconstruye LFN y detecta cadenas ciclicas o fuera de geometria.

## ext2 escribible

El serializador actualiza bitmaps, descriptores de grupo, inodos, conteos y
directorios. Los archivos pueden usar doce punteros directos y arboles
indirectos simples, dobles y triples. Se conservan symlinks inline y nombres
largos; ACL POSIX se materializa como bloque xattr de inodo.

No se presenta ext4 como ext2: extents, checksums de metadata y el journal JBD2
pertenecen a otro formato.

## Journal y recuperacion

Cada mount escribible posee un redo journal `NCCJNL2` al final de su dispositivo.
El protocolo es explicito:

1. escribir cabecera preparada y bloques nuevos;
2. flush del journal;
3. aplicar bloques al filesystem;
4. flush de datos;
5. limpiar journal y volver a hacer flush.

`recover_mount` reproduce una transaccion preparada. La inyeccion de fallo
`AfterJournalCommit` y `AfterTargetBlocks(N)` prueba corte antes y durante la
aplicacion. `unmount` rechaza mounts dirty: el programador debe llamar
`sync_mount` o descartar de forma explicita.

## Cache y persistencia

La block cache mantiene LRU, write-back, flush, invalidacion y contadores. El
kernel autonomo sincroniza primero el filesystem y despues envia la imagen
persistente al namespace NVMe mediante comandos Write y Flush; no hay
writeback oculto en el destructor.

## Evidencia

Las pruebas cubren FAT12/16/32, LFN, ext2 directo/simple/doble/triple, ACL,
journal, recuperacion, remontaje y verificacion CLI. La ejecucion host valida
algoritmos y bytes de dispositivo; una placa fisica solo queda certificada por
su runner y certificado independiente.
