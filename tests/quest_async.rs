use crate::common;

use std::path::PathBuf;
use wow_ui_sim::loader::{find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn env_with_object_api() -> WowLuaEnv {
    let env = common::env_with_shared_xml();
    let ui = blizzard_ui_dir();

    let colors_toc = find_toc_file(&ui.join("Blizzard_Colors"))
        .expect("Blizzard_Colors TOC should exist");
    load_addon(&env.loader_env(), &colors_toc).expect("Failed to load Blizzard_Colors");

    let object_api_toc = find_toc_file(&ui.join("Blizzard_ObjectAPI"))
        .expect("Blizzard_ObjectAPI TOC should exist");
    load_addon(&env.loader_env(), &object_api_toc).expect("Failed to load Blizzard_ObjectAPI");

    env
}

#[test]
fn request_load_quest_by_id_fires_result_event() {
    let env = env_with_object_api();

    let (seen, quest_id, success): (bool, i32, bool) = env
        .eval(
            r#"
            local seen, got_id, got_success = false, 0, false
            local f = CreateFrame("Frame")
            f:RegisterEvent("QUEST_DATA_LOAD_RESULT")
            f:SetScript("OnEvent", function(_, event, id, ok)
                if event == "QUEST_DATA_LOAD_RESULT" then
                    seen = true
                    got_id = id
                    got_success = ok
                end
            end)

            C_QuestLog.RequestLoadQuestByID(80000)
            return seen, got_id, got_success
            "#,
        )
        .unwrap();

    assert!(seen, "QUEST_DATA_LOAD_RESULT should fire");
    assert_eq!(quest_id, 80000);
    assert!(success, "known quest IDs should report success");
}

#[test]
fn quest_event_listener_callback_runs_from_request_load() {
    let env = env_with_object_api();

    let callback_fired: bool = env
        .eval(
            r#"
            local fired = false
            QuestEventListener:AddCallback(80001, function()
                fired = true
            end)

            C_QuestLog.RequestLoadQuestByID(80001)
            return fired
            "#,
        )
        .unwrap();

    assert!(
        callback_fired,
        "QuestEventListener callbacks should run via QUEST_DATA_LOAD_RESULT"
    );
}

#[test]
fn request_load_unknown_quest_reports_failure() {
    let env = env_with_object_api();

    let (seen, quest_id, success, callback_fired): (bool, i32, bool, bool) = env
        .eval(
            r#"
            local seen, got_id, got_success = false, 0, true
            local callbackFired = false

            local f = CreateFrame("Frame")
            f:RegisterEvent("QUEST_DATA_LOAD_RESULT")
            f:SetScript("OnEvent", function(_, event, id, ok)
                if event == "QUEST_DATA_LOAD_RESULT" then
                    seen = true
                    got_id = id
                    got_success = ok
                end
            end)

            QuestEventListener:AddCallback(99999, function()
                callbackFired = true
            end)

            C_QuestLog.RequestLoadQuestByID(99999)
            return seen, got_id, got_success, callbackFired
            "#,
        )
        .unwrap();

    assert!(seen, "unknown quest requests should still resolve");
    assert_eq!(quest_id, 99999);
    assert!(!success, "unknown quest IDs should report failure");
    assert!(
        !callback_fired,
        "failed quest loads should clear callbacks without executing them"
    );
}

#[test]
fn spell_event_listener_callback_runs_from_request_load() {
    let env = env_with_object_api();

    let callback_fired: bool = env
        .eval(
            r#"
            local fired = false
            local spell = Spell:CreateFromSpellID(35395)
            spell:ContinueOnSpellLoad(function()
                fired = true
            end)

            C_Spell.RequestLoadSpellData(35395)
            return fired
            "#,
        )
        .unwrap();

    assert!(
        callback_fired,
        "SpellEventListener callbacks should run via SPELL_DATA_LOAD_RESULT"
    );
}

#[test]
fn item_event_listener_callback_runs_from_request_load() {
    let env = env_with_object_api();

    let callback_fired: bool = env
        .eval(
            r#"
            local fired = false
            local item = Item:CreateFromItemID(6948)
            item:ContinueOnItemLoad(function()
                fired = true
            end)

            C_Item.RequestLoadItemDataByID(6948)
            return fired
            "#,
        )
        .unwrap();

    assert!(
        callback_fired,
        "ItemEventListener callbacks should run via ITEM_DATA_LOAD_RESULT"
    );
}

#[test]
fn spell_cancel_callback_is_spent_after_sync_load() {
    let env = env_with_object_api();

    let (callback_fired, cancel_result): (bool, bool) = env
        .eval(
            r#"
            local fired = false
            local spell = Spell:CreateFromSpellID(35395)
            local cancel = spell:ContinueWithCancelOnSpellLoad(function()
                fired = true
            end)

            return fired, cancel()
            "#,
        )
        .unwrap();

    assert!(
        callback_fired,
        "Spell callback should fire through the ObjectAPI listener"
    );
    assert!(
        !cancel_result,
        "cancel should report false once the synchronous load already consumed the callback"
    );
}

#[test]
fn continuable_container_satisfies_mixed_item_and_spell_loads() {
    let env = env_with_object_api();

    let (callback_count, outstanding_after): (i32, i32) = env
        .eval(
            r#"
            local callbackCount = 0
            local container = ContinuableContainer:Create()
            container:AddContinuable(Item:CreateFromItemID(6948))
            container:AddContinuable(Spell:CreateFromSpellID(35395))
            container:ContinueOnLoad(function()
                callbackCount = callbackCount + 1
            end)

            return callbackCount, container:GetNumOutstandingLoads()
            "#,
        )
        .unwrap();

    assert_eq!(
        callback_count, 1,
        "ContinuableContainer should resolve once mixed item/spell loads complete"
    );
    assert_eq!(
        outstanding_after, 0,
        "ContinuableContainer should not leave outstanding loads after completion"
    );
}
