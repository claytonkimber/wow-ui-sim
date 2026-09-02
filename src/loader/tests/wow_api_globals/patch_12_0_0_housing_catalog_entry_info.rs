//! Retail 12.0.0 HousingCatalogEntryInfo field removal.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_housing_catalog_entry_info_removes_num_stored() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local info = C_HousingCatalog.GetCatalogEntryInfo(1001)
                if type(info) ~= "table" then
                    return "entry=" .. type(info)
                end
                if info.numStored ~= nil then
                    return "numStored=" .. tostring(info.numStored)
                end
                if type(info.totalNumStored) ~= "number" then
                    return "totalNumStored=" .. type(info.totalNumStored)
                end
                if type(info.totalNumPlaced) ~= "number" then
                    return "totalNumPlaced=" .. type(info.totalNumPlaced)
                end
                if type(info.destroyableInstanceCount) ~= "number" then
                    return "destroyableInstanceCount=" .. type(info.destroyableInstanceCount)
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 HousingCatalogEntryInfo should omit numStored and publish replacement counts"
    );
}
