# Retired Blizzard_TransmogShared inventory-slot scope

A temporary loader scope exposed removed `GetInventorySlotInfo` to `Blizzard_TransmogShared` based on stale 12.0.7 vendor source. It was retired after the retail UI manifest and cache refreshed to 12.1.0.69497.

## Historical symptom

The stale cached `Blizzard_TransmogShared.lua` called `GetInventorySlotInfo` while loading. Publishing that function globally would have violated retail 12.1 removal behavior, so the loader temporarily installed a target-scoped environment.

## Root cause

The profile cache accepted any usable existing file without comparing build or manifest identity. It retained 12.0.7 source after the installed retail client and Gethe `live` source reached 12.1.0.69497.

Current `Blizzard_TransmogShared.lua` calls `C_PaperDollInfo.GetInventorySlotInfo` directly. The namespaced API is registered normally; the removed legacy global remains nil on retail 12.1. No retained loader environment is needed.

## Retirement verification

- `GetInventorySlotInfo` is nil before and after `Blizzard_TransmogShared` loads.
- `TransmogUtil.GetTransmogLocation("HEADSLOT", ...)` remains callable after runtime and direct loads.
- Cache provenance now includes the active client build and compiled manifest identity, so a changed identity refreshes the profile cache before loading.

## Sources

- [retail.txt](../../../data/blizzard-ui-files/retail.txt) — refreshed retail UI manifest
- [inventory_slot.rs](../../../src/lua_api/globals/inventory_slot.rs) — legacy global registration and namespaced backing function
- [c_paper_doll_info.rs](../../../src/c_api/c_paper_doll_info.rs) — current namespaced API registration
- [blizzard_transmog_shared_loads.rs](../../../tests/blizzard_transmog_shared_loads.rs) — public-removal and TransmogUtil behavior coverage
- [updating-blizzard-ui-to-a-new-patch.md](../../../docs/updating-blizzard-ui-to-a-new-patch.md) — source refresh workflow

## See Also

- [[addon-loading]] — runtime LoadOnDemand execution
- [[lua-api]] — public Lua API surface
