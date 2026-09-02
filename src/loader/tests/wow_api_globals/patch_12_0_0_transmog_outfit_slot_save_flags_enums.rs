//! Focused retail 12.0.0 transmog outfit slot-save flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_slot_save_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local flags = Enum.TransmogOutfitSlotSaveFlags
                if type(flags) ~= "table" then
                    return "TransmogOutfitSlotSaveFlags: expected table"
                end
                local count = 0
                for name, value in pairs(flags) do
                    count = count + 1
                    if name ~= "AppearanceIsNotValid"
                        or type(value) ~= "number"
                        or value ~= 1 then
                        return "unexpected member"
                    end
                end
                if count ~= 1 then
                    return "member count: expected 1, got " .. tostring(count)
                end

                local meta = Enum.TransmogOutfitSlotSaveFlagsMeta
                if type(meta) ~= "table" then
                    return "TransmogOutfitSlotSaveFlagsMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 1
                    or meta.MaxValue ~= 1
                    or meta.NumValues ~= 1 then
                    return "metadata mismatch"
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog outfit slot-save flags enum mismatch: {result}"
    );
}
