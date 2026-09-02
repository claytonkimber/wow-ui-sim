# Lua Bytecode Cache Growth Bound

Commit `39caf2662` bounded the packed Lua bytecode cache after a valid cache grew beyond available memory during isolated addon tests. The fix rejects oversized packs before payload allocation and enforces the serialized size limit on every store, while preserving warm-cache reuse when compaction can fit.

## Content

### Symptoms and forensic evidence

An isolated `Blizzard_AchievementUI` smoke test stalled while loading the first baseline addon, `Blizzard_SharedXMLBase`. The cache pack for the clean proof checkout was **32,316,662,045 bytes** (~30.1 GiB), with a valid `WOWBC002` header and whitelist version. A bounded scan found **25,256,275 entries** and **25,256,274 unique hashes**; the pack parsed through exact EOF with no torn entry or trailing garbage. Growth was therefore millions of distinct appended entries, not ordinary duplicate accumulation.

### Root cause

`MAX_PACK_SIZE` was enforced only when a process first loaded an existing pack. The loader called `read_to_end` before checking the file size, so an oversized valid pack could be materialized in memory before compaction or deletion. During the process lifetime, `put()` and legacy-key promotion appended entries without checking the serialized pack size, and mutated in-memory state before persistence succeeded. Repeated generated chunks could therefore grow both the file and `CacheState` without a bound.

### Fix

`39caf2662` applies the bound at both load and store boundaries:

- inspect file metadata and perform a bounded read before parsing an existing pack;
- calculate serialized header/entry size before append;
- compact live entries before adding a replacement when the append would exceed the limit;
- rebuild with only the new entry when the compacted live set still cannot fit;
- reject entries larger than the limit;
- persist replacement data before replacing `CacheState`, and roll back failed appends;
- route legacy-key promotion through the same bounded store path.

This keeps the warm full-addon cache path while preventing unbounded in-process growth. It does not claim a broader cache eviction policy or a new generated-chunk persistence policy.

### Proof

Focused tests live in `src/loader/bytecode_cache.rs`:

- `bounded_store_rejects_oversized_pack_before_payload_read`
- `bounded_store_compacts_stale_entries_before_adding_new_entry`
- `bounded_store_rebuilds_with_only_new_entry_when_unique_set_exceeds_limit`
- `bounded_store_rejects_entry_larger_than_pack_limit`
- `bounded_store_failed_append_leaves_in_memory_state_unchanged`
- `bounded_store_legacy_lookup_promotion_uses_same_limit`

Existing pack-header, torn-entry, compaction, and discard tests remain in the same module. The failing isolated loader boundary was reproduced with `WOW_SIM_TRACE_LOAD_ADDON=1`; after the fix, the focused cache tests provide the bounded-read, serialized-cap, compaction/rebuild, promotion, and persistence-order proof.

## Sources

- [bytecode_cache.rs](../../../src/loader/bytecode_cache.rs) — packed cache format, bounded load/store paths, and focused regression tests
- [Track 3 global-slot ABI](../design/track-3-global-slot-abi.md) — bytecode-cache versioning and slot-ABI invalidation context

## See Also

- [[track-3-global-slot-abi]] — cache versioning protects the global-slot ABI; this page covers size and persistence bounds
- [[talent-performance]] — earlier startup profiling identified cold bytecode-cache reuse as a performance concern
