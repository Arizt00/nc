# nC / ncc — F2 [@machine] Inline Labels + Local Flow Closure

## Estado

La subfase de labels inline y control local básico de flujo dentro de `[@machine]` queda cerrada funcionalmente.

Esta subfase permite declarar etiquetas directamente dentro de la sección `inst:` sin depender exclusivamente de la sección `label:`.

También introduce validación semántica mínima de terminadores locales dentro de `inst:`.

## Capacidades validadas

### 1. Labels inline en `inst:`

```n
machineInlineLabelOk[@machine] {
    reg: ax;
    inst:
        entryPoint:
        call -> worker;
        jmp endPoint;

        worker:
        inc ax;

        endPoint:
    final:
        ret;
}
