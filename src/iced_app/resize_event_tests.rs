use super::*;
use crate::iced_app::app::AppInit;
use crate::screen::ScreenKind;
use crate::texture::TextureManager;
use iced::Size;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc;

fn build_test_app() -> App {
    let env = Rc::new(RefCell::new(
        WowLuaEnv::new().expect("Failed to create Lua environment"),
    ));
    env.borrow().set_screen_mode(ScreenKind::Game);

    let texture_manager = Rc::new(RefCell::new(TextureManager::new()));
    let font_system = Rc::new(RefCell::new(crate::render::WowFontSystem::new()));
    let glyph_atlas = Rc::new(RefCell::new(crate::render::GlyphAtlas::new()));
    let (_cmd_tx, cmd_rx) = mpsc::channel(1);
    let (_lua_tx, lua_rx) = std::sync::mpsc::channel();

    App::build_app(AppInit {
        env,
        log_messages: Vec::new(),
        texture_manager,
        font_system,
        glyph_atlas,
        cmd_rx,
        lua_rx,
        debug_borders: false,
        debug_anchors: false,
        saved_vars: None,
        config: crate::config::SimConfig::default(),
    })
}

// Retail (12.0.5, live probe via docs/addons/ScaleEventProbe) fires
// DISPLAY_SIZE_CHANGED then UI_SCALE_CHANGED as an ordered pair on every
// display/scale recalculation; neither event ever fires alone. See
// docs/wiki/investigations/display-size-ui-scale-events.md.
#[test]
fn resizing_window_fires_display_size_changed_then_ui_scale_changed() {
    let app = build_test_app();
    app.env.borrow().set_screen_size(800.0, 600.0);
    app.env
        .borrow()
        .exec(
            r#"
            __event_order = {}
            local frame = CreateFrame("Frame")
            frame:RegisterEvent("DISPLAY_SIZE_CHANGED")
            frame:RegisterEvent("UI_SCALE_CHANGED")
            frame:SetScript("OnEvent", function(_, event)
                table.insert(__event_order, event)
            end)
            "#,
        )
        .expect("event counter setup should succeed");

    app.sync_screen_size_to_state(Size::new(1024.0, 768.0));

    let order: String = app
        .env
        .borrow()
        .eval("return table.concat(__event_order, ',')")
        .expect("event order should be readable");
    assert_eq!(
        order, "DISPLAY_SIZE_CHANGED,UI_SCALE_CHANGED",
        "window resize should fire the retail event pair, display-first"
    );
}

#[test]
fn ui_scale_cvar_change_fires_pair_before_cvar_update() {
    let app = build_test_app();
    app.env
        .borrow()
        .state()
        .borrow()
        .cvars
        .set("useUiScale", "1");
    app.env
        .borrow()
        .exec(
            r#"
            local function captureScaleCVarOrder(cvarName, requestedValue, expectedScale)
                local events = {}
                local oldValue = GetCVar(cvarName)
                local frame = CreateFrame("Frame")
                frame:RegisterEvent("DISPLAY_SIZE_CHANGED")
                frame:RegisterEvent("UI_SCALE_CHANGED")
                frame:RegisterEvent("CVAR_UPDATE")
                frame:SetScript("OnEvent", function(_, event, name, value)
                    local pairStateMatches = GetCVar(cvarName) == oldValue
                        and math.abs(UIParent:GetEffectiveScale() - expectedScale) < 0.0001
                    local cvarStateMatches = name == cvarName
                        and tostring(value) == tostring(requestedValue)
                        and GetCVar(cvarName) == tostring(requestedValue)
                    table.insert(events, table.concat({
                        event,
                        tostring(event == "CVAR_UPDATE" and cvarStateMatches or pairStateMatches),
                    }, ":"))
                end)

                SetCVar(cvarName, requestedValue)
                frame:UnregisterAllEvents()
                return table.concat(events, "|")
            end

            __ui_scale_order = captureScaleCVarOrder("uiScale", 0.8, 0.8)
            __use_ui_scale_order = captureScaleCVarOrder("useUiScale", 0, 0.8)
            "#,
        )
        .expect("UI scale CVar probe should succeed");

    let order: String = app
        .env
        .borrow()
        .eval("return __ui_scale_order .. ';' .. __use_ui_scale_order")
        .expect("UI scale CVar event order should be readable");
    assert_eq!(
        order,
        "DISPLAY_SIZE_CHANGED:true|UI_SCALE_CHANGED:true|CVAR_UPDATE:true;\
         DISPLAY_SIZE_CHANGED:true|UI_SCALE_CHANGED:true|CVAR_UPDATE:true"
    );
}

// Retail fires the display/scale pair before PLAYER_LOGIN and never after it
// at startup (live probe, see the wiki investigation page).
#[test]
fn startup_fires_display_scale_pair_before_player_login_not_after() {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_mode(ScreenKind::Game);
    env.exec(
        r#"
        __startup_seq = {}
        local frame = CreateFrame("Frame")
        frame:RegisterEvent("DISPLAY_SIZE_CHANGED")
        frame:RegisterEvent("UI_SCALE_CHANGED")
        frame:RegisterEvent("PLAYER_LOGIN")
        frame:SetScript("OnEvent", function(_, event)
            table.insert(__startup_seq, event)
        end)
        "#,
    )
    .expect("startup sequence recorder should install");

    crate::startup::fire_startup_events_headless(&env);

    let seq: String = env
        .eval("return table.concat(__startup_seq, ',')")
        .expect("startup sequence should be readable");
    assert!(
        seq.contains("DISPLAY_SIZE_CHANGED,UI_SCALE_CHANGED,PLAYER_LOGIN"),
        "pair should fire immediately before PLAYER_LOGIN, got: {seq}"
    );
    let after_login = seq.split("PLAYER_LOGIN").nth(1).unwrap_or("");
    assert!(
        !after_login.contains("DISPLAY_SIZE_CHANGED") && !after_login.contains("UI_SCALE_CHANGED"),
        "pair must not fire after PLAYER_LOGIN at startup, got: {seq}"
    );
}
