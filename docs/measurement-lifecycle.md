# Measurement driver failure and portability closure

The prior collectors could reject a new invocation while leaving the previous
successful context untouched. Two independent failing fixtures retain that
observation in [the before log](evidence/measurement-lifecycle/before-failure.log).
No old measurement is relabeled as a failed or successful new attempt.

[ADR 0048](decisions/0048-source-checked-measurement-attempts.md) records the new
contract: start a running journal before validation/build, retain immutable
attempt directories and partial complete cases, check source/binary/input
stability, validate comparability before publication, then atomically publish
complete status with the raw hash. A process killed without cleanup remains
running. One writer per output directory is required; power-loss durability
and distributed locking are not claimed.

The source-auditor synthetic fixture now writes explicit UTF-8/LF bytes and
hashes the actual file. The first process interval is retained as cold_ns rather
than discarded; all 31 subsequent positive intervals remain. Go preparation
validates all five clean pins before publication and records preparing/failed/
complete state and generated-input hashes. Tests inject copy/replace failures,
invalid pins, invalid intervals, duplicate cases/metadata, interrupts and changed
captured metadata/source/raw inputs. Captured graph extraction binds the entire
source manifest to the completed compiler run and revalidates source/raw facts
at the end; the whole graph must match. Compiler work is outside extraction time.

The native source c5069e0 passes 126 Python tests in normal and optimized modes;
18 independent driver test methods are included. Native complete qualification
passes 18 subprocess stages: both auditors before/after, the two native compiler
captures, 30 remaining paths (including all sixteen example processes), four
balanced reader runs, prepared Go module/tests and all six historical reporting
CLIs. [Retained native contexts/logs and hashes](evidence/measurement-lifecycle/native-context.json).
All times in this cohort are excluded from performance conclusions.

The dedicated [three-OS/MSRV run](https://github.com/n3dypop/Matea/actions/runs/39126579378)
passes all seven required jobs. [Independent artifact binding](evidence/measurement-lifecycle/ci-source-bindings.json)
verifies all 147 known inputs, 18 qualification logs and the complete per-host
measurement/reader records. All 26 platform-neutral remaining-path outputs match
across hosts. Two compiler graphs are intentionally native profiles; Python
coverage CLI stdout has native EOL, and full reports include current host/toolchain
metadata. Full removed-metadata report equivalence is not independently claimed.
The first verification attempt used the wrong uploaded LLVM subdirectory; the
actual quality/ path was then verified, without changing any CI result.

Behavior and portability for this tool unit move from 50 to 75 in the bounded
review scope; 52 low assessments become 50. Performance remains 50. No threshold
or capability scope is relaxed. The qualification does not recollect or close
all twenty API profiles. Current assessment-only quality-plan changes follow
c5069e0; its source snapshot remains historical and is never rewritten to imply
a later source was tested. The next implementation validation refreshes the
whole native report. Other reporter/preparation performance workloads, resident
memory and remaining reference coverage require work; Stage A, Stage B and F5
stay closed.
