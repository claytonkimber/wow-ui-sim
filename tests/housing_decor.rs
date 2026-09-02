use wow_ui_sim::lua_api::WowLuaEnv;

const HOUSING_DECOR_PREVIEW_STATE_SCRIPT: &str = r#"
    if C_HousingDecor.IsPreviewState() ~= false or C_HousingDecor.GetNumPreviewDecor() ~= 0 then
        return "wrong_initial_preview_state"
    end

    C_HousingDecor.EnterPreviewState()
    if C_HousingDecor.IsPreviewState() ~= true or C_HousingDecor.GetNumPreviewDecor() ~= 1 then
        return "enter_preview_not_reflected"
    end

    C_HousingDecor.ExitPreviewState()
    if C_HousingDecor.IsPreviewState() ~= false or C_HousingDecor.GetNumPreviewDecor() ~= 0 then
        return "exit_preview_not_reflected"
    end

    return "ok"
"#;

const HOUSING_DECOR_SCRIPT: &str = r#"
    if C_HousingDecor.IsDecorSelected() ~= true then
        return "selected_decor_not_reported"
    end

    local info = C_HousingDecor.GetSelectedDecorInfo()
    if not info then
        return "missing_selected_decor_info"
    end

    if info.decorGUID ~= "Decor-Selection-2001" then
        return "wrong_selected_guid:" .. tostring(info.decorGUID)
    end

    if info.name ~= "Azure Reading Lamp" then
        return "wrong_selected_name:" .. tostring(info.name)
    end

    if info.canBeRemoved ~= true then
        return "wrong_selected_removable_state"
    end

    if info.isLocked ~= false then
        return "wrong_selected_locked_state"
    end

    local hyperlink = C_HousingDecor.GetDecorHyperlink(91002)
    if hyperlink ~= "|cff66bbff|Hhousingdecor:91002|h[Azure Upholstery]|h|r" then
        return "wrong_hyperlink:" .. tostring(hyperlink)
    end

    if C_HousingDecor.GetDecorHyperlink(99999) ~= nil then
        return "unknown_hyperlink_should_be_nil"
    end

    return "ok"
"#;

fn env() -> WowLuaEnv {
    WowLuaEnv::new().expect("Failed to create Lua environment")
}

#[test]
fn housing_decor_preview_state_round_trips() {
    let env = env();
    let result: String = env
        .eval(HOUSING_DECOR_PREVIEW_STATE_SCRIPT)
        .expect("seeded C_HousingDecor preview state should be queryable");
    assert_eq!(result, "ok");
}

#[test]
fn housing_decor_selection_and_hyperlink_methods_use_seeded_state() {
    let env = env();
    let result: String = env
        .eval(HOUSING_DECOR_SCRIPT)
        .expect("seeded C_HousingDecor methods should be queryable");
    assert_eq!(result, "ok");
}
