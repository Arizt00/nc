# Machine EVEX Prefix Fields

EVEX is encoded as four prefix bytes before opcode and ModRM/SIB:

- byte 0: `0x62`.
- P0: inverted `R`, `X`, `B`, `R'`, and opcode map.
- P1: `W`, inverted low `vvvv`, fixed EVEX bit, and `pp`.
- P2: `z`, `L'L`, contextual `b`, inverted high `V'`, and `aaa`.

The Phase 3 encoder computes these fields from the destination register, source register, memory base/index, vector length, mask, zeroing, broadcast, and rounding/SAE decorators.
