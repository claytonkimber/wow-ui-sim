//! Focused retail 12.0.0 transmog outfit slot metadata.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_slot_metadata_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local slots = Enum.TransmogOutfitSlot
                if type(slots) ~= "table" then
                    return "TransmogOutfitSlot: expected table"
                end

                local count = 0
                for _, value in pairs(slots) do
                    count = count + 1
                    if type(value) ~= "number" then
                        return "TransmogOutfitSlot: expected numeric members"
                    end
                end
                if count ~= 15 then
                    return "member count: expected 15, got " .. tostring(count)
                end

                local meta = Enum.TransmogOutfitSlotMeta
                if type(meta) ~= "table" then
                    return "TransmogOutfitSlotMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 14
                    or meta.NumValues ~= 15 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog outfit slot metadata mismatch: {result}"
    );
}
