# nC v1.0.0 — Guía de modos

## Modo `[@]`

- Superficie soberana general.
- Fixture de referencia: `examples/l2k_s8c_runtime_inspect_runtime_paths_ok.n`.

## Modo `[@machine]`

- ISA tipada y reglas de flujo/terminación.
- Fixture de referencia: `examples/machine/legacy_at/machine_at_strict_ok.n`.

## Modo `primitive[@]`

- Contratos materiales tipados y puentes root/machine.
- Fixture de referencia: `examples/primitive_at/primitive_at_boot_serial_material_ok.n`.

## Modo `[@inspect]`

- Consulta runtime canónica controlada por contrato de imports.
- Fixture de referencia: `examples/inspect_at/inspect_at_runtime_contract_ok.n`.

## Modo `[@security]`

- Assets/policies/enforcement operativo observable.
- Fixture de referencia: `examples/security_at/security_at_runtime_allow_ok.n`.

## Modo `[@network]`

- Flujo de conexión operable con estados/errores canónicos.
- Fixture de referencia: `examples/network_at/network_at_flow_contract_ok.n`.

## Modo `[@system]`

- Coordinación de lifecycle/health/degraded/recovery.
- Fixture de referencia: `examples/system_at/system_at_lifecycle_ok.n`.

## Modo `[@tooling]`

- Contrato operativo de pipeline de tooling.
- Fixture de referencia: `examples/tooling_at/tooling_at_lifecycle_ok.n`.

## Modo `[@compiler]`

- Capa operativa parse/check/materialize.
- Fixture de referencia: `examples/compiler_at/compiler_at_lifecycle_ok.n`.

## Modo `[@kernel]`

- Modelo mínimo serio de scheduler/control/quarantine/panic.
- Fixture de referencia: `examples/kernel_at/kernel_at_lifecycle_ok.n`.

## Modo `[@ai]`

- Orquestación tipada de session/model/agent/chat/memory.
- Fixture de referencia: `examples/ai_at/ai_at_lifecycle_ok.n`.
