//! Focused retail 12.0.0 Pet Journal, spell overlay, and tracker CVar tests.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_pet_journal_overlay_tracker_cvar_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    {"petJournalFilters", "0"},
                    {"petJournalSourceFilters", "0"},
                    {"petJournalTypeFilters", "0"},
                    {"spellActivationOverlayOpacity", "0.650000"},
                    {"superTrackerDist", "0.750000"},
                }
                for index, entry in ipairs(expected) do
                    local name = entry[1]
                    local expected_value = entry[2]
                    local value = GetCVar(name)
                    local default_value = GetCVarDefault(name)
                    if value ~= expected_value or default_value ~= expected_value then
                        return index .. ":" .. name .. ": expected current/default " .. expected_value
                            .. ", got " .. tostring(value)
                            .. "/" .. tostring(default_value)
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 Pet Journal/overlay/tracker CVar mismatch: {result}"
    );
}
