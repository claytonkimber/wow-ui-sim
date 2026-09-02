//! Focused retail 12.0.0 housing decor placement restriction values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_housing_decor_placement_restriction_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    TooFarAway = 1,
                    OutsideRoomBounds = 2,
                    OutsidePlotBounds = 4,
                    ChildOutsideBounds = 8,
                    InvalidTarget = 16,
                    InvalidCollision = 32,
                }
                local restriction = Enum.HousingDecorPlacementRestriction
                if type(restriction) ~= "table" then
                    return "HousingDecorPlacementRestriction: expected table"
                end

                local count = 0
                for name, value in pairs(restriction) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 6 then
                    return "member count: expected 6, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if restriction[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end
                if restriction.NotInsideRoom ~= nil then
                    return "NotInsideRoom: expected removed member"
                end

                local meta = Enum.HousingDecorPlacementRestrictionMeta
                if type(meta) ~= "table" then
                    return "HousingDecorPlacementRestrictionMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 1
                    or meta.MaxValue ~= 32
                    or meta.NumValues ~= 6 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 housing decor placement restriction mismatch: {result}"
    );
}
