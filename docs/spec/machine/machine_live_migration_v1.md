# Machine persistence and live migration v1

## Persistent snapshot

`NCSNAP` stores a versioned VM identity, generation, power state, architecture, target, full or
incremental memory, every vCPU, pending interrupts, versioned devices, virtual-clock state and metadata.
Encoding validates the complete object before writing. Persistence uses a same-directory temporary
file, file sync, atomic rename and directory sync on Unix.

Memory pages are ordered and unique. Full snapshots cover the entire declared RAM; incremental
snapshots accept only in-range pages with exact last-page length and CRC. Decode is bounded and rejects
duplicates, malformed lengths, zero schema versions, NIL identities and checksum mismatch.

## Dirty tracking

`DirtyPageTracker` supports explicit writes, KVM dirty bitmaps and KVM dirty-ring GFNs. Every drain
increments a generation and emits sorted pages. Dirty pages produced while pre-copy is running remain
eligible for the next round; stop-and-copy always drains the final generation.

## Wire protocol

`NCMIGR01` is a bounded binary frame protocol over any `Read + Write` transport. The TCP path performs:

1. capability negotiation;
2. ordered pre-copy rounds;
3. source pause and final dirty-page transfer;
4. vCPU, pending-interrupt, device and clock transfer;
5. destination validation and prepare;
6. commit-token verification;
7. destination commit acknowledgement;
8. source commit, or rollback on any failure.

Architecture, target, page size, RAM size, vCPU count, required features and each device schema range
must agree. The source cannot resume after destination commit without the matching token, preventing
split brain.

## Time continuity

The clock policy is explicit: freeze during transfer or advance by measured downtime. Counter value,
monotonic time, wall time and accumulated downtime are checked together. All restored vCPUs receive the
same resumed guest-monotonic time.

## Certification boundary

`certify-migration` exercises real TCP, dirty generations, multi-vCPU state, device versions, pending
interrupts, clock continuity, two-phase commit and a persistent restored snapshot. It does not claim
physical cross-host VM execution unless source and destination adapters are run on separate certified
hosts. The certificate records this boundary as `physical_cross_host_execution`.
Snapshot bundles remain verifiable after relocation; their certificate resolves retained artifacts
relative to itself whenever the original absolute bundle root is unavailable.
