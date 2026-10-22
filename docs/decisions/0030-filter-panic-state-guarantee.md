# ADR 0030: Preserve a valid collection state after a custom filter panic

Status: accepted for the implemented Foundation collection API.

`List::filter_with` accepts application matching code. Clearing visible items before resetting selection left an old out-of-range position when the predicate panicked and the caller caught it. A regression reproduces selection `None` despite a nonempty accepted prefix; scrolling could hide the remaining prefix. This is a state-consistency defect, not an unsafe-memory access claim.

Reset selection before invoking user code. If a predicate unwinds, its accepted prefix remains visible and selects its first item, or has no selection when empty. This is an explicit basic panic guarantee. Preserving the entire prior filter would require temporary storage and a different rollback contract; no such guarantee is introduced. The successful filter performs the same matching work and retains its allocation reuse. No speed improvement is claimed for moving this store.

The regression tests every panic position in a four-item input, catches unwind, inspects rendering/selection and subsequently filters/navigates again. Public API tests also cover empty and nonempty lists, zero-height rows, filtered original indices, boundary navigation, inactive/released/modified input, ragged/empty tables, and forms with zero to five fields. Form submission revisits all invalid fields, focuses the first failure, and editing clears submitted state. These tests cover implemented behavior; the separate Bubbles/Huh reference coverage and performance verdicts remain incomplete.
