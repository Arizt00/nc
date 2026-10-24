# Suspensión hosted

`sleepMs`, `sleepSec` y `yieldRuntime` están disponibles desde el prelude, sin
import visible.

```n
sleepMs(10);
sleepSec(1);
yieldRuntime();
```

Las llamadas son bloqueantes y explícitas. No crean futures, tareas ni
scheduler. Duraciones negativas se convierten en espera cero. Las duraciones
grandes se dividen para respetar la anchura de la API del sistema.

En kernel, Machine y perfiles freestanding no se introduce esta familia hosted;
el programador debe usar temporizadores e instrucciones del target.
