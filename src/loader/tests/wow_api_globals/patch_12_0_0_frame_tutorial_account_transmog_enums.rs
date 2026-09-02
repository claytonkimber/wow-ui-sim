//! Focused retail 12.0.0 FrameTutorialAccount transmog additions.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_frame_tutorial_account_transmog_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local account = Enum.FrameTutorialAccount
                if type(account) ~= "table" then
                    return "FrameTutorialAccount: expected table"
                end

                local expected = {
                    TransmogOutfits = 41,
                    TransmogSets = 42,
                    TransmogCustomSets = 43,
                    TransmogSituations = 44,
                    TransmogWeaponOptions = 45,
                }
                for name, expected_value in pairs(expected) do
                    local value = account[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end

                local count = 0
                local seen_values = {}
                for _, value in pairs(account) do
                    count = count + 1
                    if type(value) ~= "number" or value < 1 or value > 45 then
                        return "unexpected member value " .. tostring(value)
                    end
                    if seen_values[value] then
                        return "duplicate member value " .. tostring(value)
                    end
                    seen_values[value] = true
                end
                if count ~= 45 then
                    return "member count: expected 45, got " .. tostring(count)
                end

                local meta = Enum.FrameTutorialAccountMeta
                if type(meta) ~= "table" then
                    return "FrameTutorialAccountMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 1
                    or meta.MaxValue ~= 45
                    or meta.NumValues ~= 45 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 FrameTutorialAccount transmog enum mismatch: {result}"
    );
}
