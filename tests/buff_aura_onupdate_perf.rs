use crate::common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use wow_ui_sim::loader::{find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;

const AURA_ONUPDATE_ADDONS: &[(&str, &str)] = &[
    ("Blizzard_SharedXMLBase", "Blizzard_SharedXMLBase.toc"),
    ("Blizzard_Colors", "Blizzard_Colors_Mainline.toc"),
    ("Blizzard_SharedXML", "Blizzard_SharedXML_Mainline.toc"),
    (
        "Blizzard_SharedXMLGame",
        "Blizzard_SharedXMLGame_Mainline.toc",
    ),
    (
        "Blizzard_UIPanelTemplates",
        "Blizzard_UIPanelTemplates_Mainline.toc",
    ),
    (
        "Blizzard_FrameXMLBase",
        "Blizzard_FrameXMLBase_Mainline.toc",
    ),
    ("Blizzard_LoadLocale", "Blizzard_LoadLocale.toc"),
    ("Blizzard_Fonts_Shared", "Blizzard_Fonts_Shared.toc"),
    ("Blizzard_HelpPlate", "Blizzard_HelpPlate.toc"),
    (
        "Blizzard_AccessibilityTemplates",
        "Blizzard_AccessibilityTemplates.toc",
    ),
    ("Blizzard_ObjectAPI", "Blizzard_ObjectAPI_Mainline.toc"),
    ("Blizzard_UIParent", "Blizzard_UIParent_Mainline.toc"),
    ("Blizzard_TextStatusBar", "Blizzard_TextStatusBar.toc"),
    ("Blizzard_MoneyFrame", "Blizzard_MoneyFrame_Mainline.toc"),
    ("Blizzard_POIButton", "Blizzard_POIButton.toc"),
    ("Blizzard_Flyout", "Blizzard_Flyout.toc"),
    ("Blizzard_StoreUI", "Blizzard_StoreUI_Mainline.toc"),
    ("Blizzard_MicroMenu", "Blizzard_MicroMenu_Mainline.toc"),
    ("Blizzard_EditMode", "Blizzard_EditMode.toc"),
    ("Blizzard_Minimap", "Blizzard_Minimap_Mainline.toc"),
    ("Blizzard_BuffFrame", "Blizzard_BuffFrame.toc"),
    ("Blizzard_GarrisonBase", "Blizzard_GarrisonBase.toc"),
    ("Blizzard_GameTooltip", "Blizzard_GameTooltip_Mainline.toc"),
    (
        "Blizzard_UIParentPanelManager",
        "Blizzard_UIParentPanelManager_Mainline.toc",
    ),
    (
        "Blizzard_Settings_Shared",
        "Blizzard_Settings_Shared_Mainline.toc",
    ),
    (
        "Blizzard_SettingsDefinitions_Shared",
        "Blizzard_SettingsDefinitions_Shared.toc",
    ),
    (
        "Blizzard_SettingsDefinitions_Frame",
        "Blizzard_SettingsDefinitions_Frame_Mainline.toc",
    ),
    (
        "Blizzard_FrameXMLUtil",
        "Blizzard_FrameXMLUtil_Mainline.toc",
    ),
    ("Blizzard_ItemButton", "Blizzard_ItemButton_Mainline.toc"),
    ("Blizzard_QuickKeybind", "Blizzard_QuickKeybind.toc"),
    ("Blizzard_FrameXML", "Blizzard_FrameXML_Mainline.toc"),
    (
        "Blizzard_UIPanels_Game",
        "Blizzard_UIPanels_Game_Mainline.toc",
    ),
    ("Blizzard_TokenUI", "Blizzard_TokenUI.toc"),
    ("Blizzard_ActionBar", "Blizzard_ActionBar_Mainline.toc"),
];

const PER_SAMPLE_TICKS: u32 = 256;
const SAMPLE_WINDOWS: usize = 8;
const AURA_ONUPDATE_MAX_BUDGET: Duration = Duration::from_micros(500);
const AURA_ONUPDATE_HARNESS_LUA: &str = r#"
        if not PlayerFrame then
            PlayerFrame = CreateFrame("Frame", "PlayerFrame", UIParent)
        end
        PlayerFrame.unit = "player"

        local container = CreateFrame("Frame", nil, UIParent)
        container.GetAuraWarningAlphaForDuration = function(self, duration)
            return 1
        end

        local button = CreateFrame("BUTTON", nil, container, "AuraButtonTemplate")
        Mixin(button, AuraButtonMixin)
        button:OnLoad()
        DEFAULT_AURA_DURATION_FONT = DEFAULT_AURA_DURATION_FONT or GameFontNormalSmall
        SMALLER_AURA_DURATION_FONT = SMALLER_AURA_DURATION_FONT or GameFontNormalSmall
        SMALLER_AURA_DURATION_OFFSET_Y = SMALLER_AURA_DURATION_OFFSET_Y or 0
        SMALLER_AURA_DURATION_FONT_MIN_THRESHOLD = 1
        SMALLER_AURA_DURATION_FONT_MAX_THRESHOLD = 100000

        button:Update({
            auraType = "Buff",
            index = 1,
            texture = 136116,
            count = 0,
            duration = 300,
            expirationTime = GetTime() + 300,
            timeMod = 1,
            auraInstanceID = 1,
        })
        local script = assert(button:GetScript("OnUpdate"), "missing OnUpdate script")
        script(button, 0.016)

        __audit_aura_button = button
        __audit_aura_onupdate = script

        function __run_aura_onupdate_ticks(count)
            local target = __audit_aura_button
            local handler = __audit_aura_onupdate
            for _ = 1, count do
                handler(target, 0.016)
            end
        end

        function __run_aura_baseline_ticks(count)
            local target = __audit_aura_button
            for _ = 1, count do
                -- Keep loop shape close to the measured path without touching visual mutators.
                local _ = target.timeLeft
            end
        end
        "#;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn load_aura_perf_env() -> WowLuaEnv {
    let env = new_aura_perf_env();
    let ui = blizzard_ui_dir();
    configure_addon_base_path(&env, &ui);
    load_aura_perf_addons(&env, &ui);
    apply_workarounds_and_bootstrap_harness(&env);
    env
}

fn new_aura_perf_env() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env
}

fn configure_addon_base_path(env: &WowLuaEnv, ui: &Path) {
    env.state().borrow_mut().addon_base_paths = vec![ui.to_path_buf()];
}

fn load_aura_perf_addons(env: &WowLuaEnv, ui: &Path) {
    for (name, toc) in AURA_ONUPDATE_ADDONS {
        load_aura_perf_addon(env, ui, name, toc);
    }
}

fn load_aura_perf_addon(env: &WowLuaEnv, ui: &Path, addon_name: &str, toc_name: &str) {
    let Some(toc_path) = resolve_addon_toc_path(ui, addon_name, toc_name) else {
        return;
    };
    if let Err(error) = load_addon(&env.loader_env(), &toc_path) {
        eprintln!("[load {addon_name}] FAILED: {error}");
    }
}

fn resolve_addon_toc_path(ui: &Path, addon_name: &str, toc_name: &str) -> Option<PathBuf> {
    let addon_dir = ui.join(addon_name);
    let requested_toc = addon_dir.join(toc_name);
    if requested_toc.exists() {
        return Some(requested_toc);
    }
    find_toc_file(&addon_dir)
}

fn apply_workarounds_and_bootstrap_harness(env: &WowLuaEnv) {
    env.apply_post_load_workarounds();
    env.exec(AURA_ONUPDATE_HARNESS_LUA)
        .expect("Failed to initialize aura onupdate perf harness");
}

fn run_tick_batch(env: &WowLuaEnv, fn_name: &str, ticks: u32) -> Duration {
    let started = Instant::now();
    env.exec(&format!("{fn_name}({ticks})"))
        .unwrap_or_else(|err| panic!("{fn_name}({ticks}) should succeed: {err}"));
    started.elapsed()
}

fn sample_aura_onupdate_cost(env: &WowLuaEnv, ticks: u32) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(SAMPLE_WINDOWS);
    for _ in 0..SAMPLE_WINDOWS {
        let baseline = run_tick_batch(env, "__run_aura_baseline_ticks", ticks);
        let measured = run_tick_batch(env, "__run_aura_onupdate_ticks", ticks);
        samples.push(measured.saturating_sub(baseline) / ticks);
    }
    samples
}

fn max_duration(samples: &[Duration]) -> Duration {
    samples.iter().copied().max().unwrap_or_default()
}

#[test]
fn buff_aura_onupdate_steady_state_stays_under_half_millisecond() {
    perf_test_timeout! {
            let env = load_aura_perf_env();
            let samples = sample_aura_onupdate_cost(&env, PER_SAMPLE_TICKS);
            let max_elapsed = max_duration(&samples);
            eprintln!(
                "buff aura OnUpdate net per-tick samples (baseline-subtracted): {:?} (max {:.2?}, budget {:.2?})",
                samples,
                max_elapsed,
                AURA_ONUPDATE_MAX_BUDGET
            );

            assert!(
                max_elapsed <= AURA_ONUPDATE_MAX_BUDGET,
                "buff aura OnUpdate max steady-state cost {:.2?} exceeds budget {:.2?}",
                max_elapsed,
                AURA_ONUPDATE_MAX_BUDGET
            );
    }
}
