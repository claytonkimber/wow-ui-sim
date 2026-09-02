//! Focused retail 12.0.0 transmog outfit slot-option metadata.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_slot_option_metadata_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local options = Enum.TransmogOutfitSlotOption
                if type(options) ~= "table" then
                    return "TransmogOutfitSlotOption: expected table"
                end

                local count = 0
                for _, value in pairs(options) do
                    count = count + 1
                    if type(value) ~= "number" then
                        return "TransmogOutfitSlotOption: expected numeric members"
                    end
                end
                if count ~= 12 then
                    return "member count: expected 12, got " .. tostring(count)
                end

                local meta = Enum.TransmogOutfitSlotOptionMeta
                if type(meta) ~= "table" then
                    return "TransmogOutfitSlotOptionMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 11
                    or meta.NumValues ~= 12 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog outfit slot-option metadata mismatch: {result}"
    );
}
