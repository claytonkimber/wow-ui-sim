use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addon_closure_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path()
        .expect("Blizzard UI cache should be available")
}

fn load_buff_frame() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Lua environment should initialize");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);
    let ui_dir = blizzard_ui_dir();
    env.state().borrow_mut().addon_base_paths = vec![ui_dir.clone()];
    wow_ui_sim::xml::register_intrinsic_templates();
    env.exec("PlayerFrame = { unit = 'player' }")
        .expect("PlayerFrame test fixture should install");

    let roots = ["Blizzard_BuffFrame"];
    for (name, toc_path) in
        discover_blizzard_addon_closure_for_screen(&ui_dir, ScreenKind::Game, &roots)
    {
        load_addon(&env.loader_env(), &toc_path)
            .unwrap_or_else(|error| panic!("[load {name}] FAILED: {error}"));
    }

    env
}

#[test]
fn aura_button_tooltip_binding_uses_filter_instance_id_and_leave_hide() {
    let env = load_buff_frame();
    let result: String = env
        .eval(
            r#"
            if type(AuraButtonMixin) ~= "table" then return "missing-aura-button-mixin" end
            if type(GameTooltip.SetUnitAura) ~= "function" then return "missing-indexed-tooltip-method" end
            if type(GameTooltip.SetUnitAuraByAuraInstanceID) ~= "function" then
                return "missing-aura-instance-tooltip-method"
            end

            PlayerFrame = { unit = "player" }
            local button = CreateFrame("Button")
            Mixin(button, AuraButtonMixin)

            local calls = {}
            GameTooltip.SetOwner = function(_, owner, anchor)
                calls.owner = owner
                calls.anchor = anchor
            end
            GameTooltip.SetFrameLevel = function(_, level)
                calls.frameLevel = level
            end
            GameTooltip.SetUnitAura = function(_, unit, index, filter)
                calls.indexed = { unit = unit, index = index, filter = filter }
            end
            GameTooltip.SetUnitAuraByAuraInstanceID = function(_, unit, auraInstanceID)
                calls.instance = { unit = unit, auraInstanceID = auraInstanceID }
            end
            GameTooltip.Hide = function()
                calls.hidden = true
            end

            button.auraType = "Buff"
            button.buttonInfo = { index = 3 }
            AuraButtonMixin.OnEnter(button)
            if calls.owner ~= button or calls.anchor ~= "ANCHOR_BOTTOMLEFT" then return "owner" end
            if calls.indexed == nil then return "helpful-call-missing" end
            if calls.indexed.unit ~= "player" or calls.indexed.index ~= 3 then return "helpful-arguments" end
            if calls.indexed.filter ~= "HELPFUL" then return "helpful-filter" end
            AuraButtonMixin.OnLeave(button)
            if calls.hidden ~= true then return "leave-hide" end

            calls.indexed = nil
            calls.hidden = nil
            button.auraType = "Debuff"
            button.buttonInfo = { index = 4 }
            AuraButtonMixin.OnEnter(button)
            if calls.indexed == nil then return "harmful-call-missing" end
            if calls.indexed.unit ~= "player" or calls.indexed.index ~= 4 then return "harmful-arguments" end
            if calls.indexed.filter ~= "HARMFUL" then return "harmful-filter" end

            calls.indexed = nil
            calls.instance = nil
            button.buttonInfo = { index = 5, auraInstanceID = 42 }
            AuraButtonMixin.OnEnter(button)
            if calls.instance == nil then return "instance-call-missing" end
            if calls.instance.unit ~= "player" or calls.instance.auraInstanceID ~= 42 then
                return "instance-arguments"
            end
            if calls.indexed ~= nil then return "instance-did-not-take-precedence" end

            return "ok"
            "#,
        )
        .expect("AuraButton tooltip binding probe should execute");

    assert_eq!(result, "ok");
}
