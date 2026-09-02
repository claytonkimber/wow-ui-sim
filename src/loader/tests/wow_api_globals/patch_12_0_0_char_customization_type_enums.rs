//! Focused retail 12.0.0 CharCustomizationType enum values and removals.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_char_customization_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Skin = 0,
                    Face = 1,
                    Hair = 2,
                    HairColor = 3,
                    FacialHair = 4,
                    CustomOptionTattoo = 5,
                    CustomOptionHorn = 6,
                    CustomOptionFacewear = 7,
                    CustomOptionTattooColor = 8,
                }
                local customization_type = Enum.CharCustomizationType
                if type(customization_type) ~= "table" then
                    return "CharCustomizationType: expected table"
                end

                local count = 0
                for name, value in pairs(customization_type) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 9 then
                    return "member count: expected 9, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if customization_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                for _, name in ipairs({"Outfit", "Facepaint", "FacepaintColor"}) do
                    if rawget(customization_type, name) ~= nil then
                        return name .. ": removed member is present"
                    end
                end

                local expected_meta = {
                    MinValue = 0,
                    MaxValue = 8,
                    NumValues = 9,
                }
                local meta = Enum.CharCustomizationTypeMeta
                if type(meta) ~= "table" then
                    return "CharCustomizationTypeMeta: expected table"
                end

                local meta_count = 0
                for name, value in pairs(meta) do
                    meta_count = meta_count + 1
                    local expected_value = expected_meta[name]
                    if expected_value == nil then
                        return "metadata." .. name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return "metadata." .. name .. ": unexpected value " .. tostring(value)
                    end
                end
                if meta_count ~= 3 then
                    return "metadata member count: expected 3, got " .. tostring(meta_count)
                end
                for name, expected_value in pairs(expected_meta) do
                    if meta[name] ~= expected_value then
                        return "metadata." .. name .. ": missing member"
                    end
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 CharCustomizationType enum mismatch: {result}"
    );
}
