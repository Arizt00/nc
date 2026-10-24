# Machine Safety Profiles

Profiles:

| Profile | Rules |
|---|---|
| userland | no `hlt`, no privileged instructions, no system registers |
| freestanding | `hlt` allowed, no OS runtime assumed |
| kernel | privileged instructions require explicit authority |
| baremetal | entry required, no runtime assumption |
| verify-only | no artifact emission |
| emit-only | emission after verifier |

Userland violations fail with `E_MACHINE_FORBIDDEN_IN_USERLAND_001` or `E_MACHINE_PRIVILEGE_001`.
