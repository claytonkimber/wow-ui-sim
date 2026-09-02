#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path()
        .expect("Blizzard UI cache should be available")
}

fn frame_effects_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_FrameEffects/Blizzard_FrameEffects.toc")
}

fn load_full_game_ui() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);

    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![blizzard_ui_dir()];
    }

    wow_ui_sim::xml::register_intrinsic_templates();

    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("[load {name}] FAILED: {err}"));
    }

    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);
    env
}

#[test]
fn blizzard_frame_effects_toc_is_eager_with_no_deps_and_allow_load_both() {
    let toc =
        TocFile::from_file(&frame_effects_toc()).expect("Blizzard_FrameEffects TOC should parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_FrameEffects has no `## LoadOnDemand` line — the GlowEmitter / \
         PowerSwirl templates and the EffectFactoryMixin pool helper are foundational \
         visual primitives consumed by NPE / soulbinds / power swirl callers, so they \
         must be available at startup before consumers reference them"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_FrameEffects does not declare `## UseSecureEnvironment` — the effect \
         factory and glow/swirl templates are pure UI primitives running in the standard \
         taint environment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_FrameEffects declares no `## Dependencies` line — the only globals \
         referenced at load time are CreateFramePool / CreateFromMixins (provided by \
         FrameXML core) and NineSlicePanelTemplate (an intrinsic template). The TOC \
         contains exactly three metadata lines (Author / Title / AllowLoad) and three \
         file lines (EffectFactory.lua / GlowEmitter.xml / PowerSwirl.xml)"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_FrameEffects declares no `## AllowLoadGameType:` line, so \
         `is_game_type_restricted()` returns false and the addon is reachable from \
         standard-retail discovery"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_FrameEffects declares no `## SavedVariables` — the EffectFactory \
         pools rebuild on every login from the templates, glow/swirl frames have no \
         persistent state"
    );

    let toc_text = std::fs::read_to_string(frame_effects_toc())
        .expect("Blizzard_FrameEffects TOC should read");
    assert!(
        !toc_text.contains("## LoadFirst"),
        "Blizzard_FrameEffects does NOT declare `## LoadFirst:` — it depends on FrameXML \
         core globals (CreateFramePool / CreateFromMixins) so the loader runs it in the \
         standard tier after FrameXML, not in the priority pre-tier"
    );
    assert!(
        toc_text.contains("## AllowLoad: Both"),
        "Blizzard_FrameEffects declares `## AllowLoad: Both` — glow/swirl effects are \
         used on glue screens too (e.g. character-create NPE flow) so the addon must \
         auto-load on Login and CharacterSelect in addition to Game"
    );
}

#[test]
fn blizzard_frame_effects_allows_all_screens_including_glue() {
    let toc =
        TocFile::from_file(&frame_effects_toc()).expect("Blizzard_FrameEffects TOC should parse");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad: Both` must allow the Game screen (src/toc.rs:307)"
    );
    assert!(
        toc.allows_screen(ScreenKind::Login),
        "`## AllowLoad: Both` must allow the Login screen — distinguishes this addon \
         from the Game-only default at src/toc.rs:311"
    );
    assert!(
        toc.allows_screen(ScreenKind::CharacterSelect),
        "`## AllowLoad: Both` must allow CharacterSelect — `is_glue()` covers all glue \
         screens"
    );
}

#[test]
fn blizzard_frame_effects_auto_loads_on_game_and_login_screens() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_FrameEffects");
    assert!(
        in_game,
        "Blizzard_FrameEffects has no `## LoadOnDemand` line and `## AllowLoad: Both`, \
         so it MUST appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_FrameEffects");
    assert!(
        in_login,
        "`## AllowLoad: Both` plus no LoadOnDemand means Blizzard_FrameEffects MUST \
         appear in Login-screen auto-discovery as well"
    );
}

prefork_full_ui_case! {
fn blizzard_frame_effects_loads_via_full_game_ui_without_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("FrameEffects")
                || message.contains("EffectFactory")
                || message.contains("GlowEmitter")
                || message.contains("PowerSwirl")
        })
        .cloned()
        .collect();

    assert!(
        load_errors.is_empty(),
        "Blizzard_FrameEffects emitted Lua errors during the full Game-screen load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_effects_is_addon_loaded_returns_true_after_full_game_ui_load(env: &WowLuaEnv) {

    let post_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_FrameEffects') and true or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        post_load,
        "After full Game-screen load, IsAddOnLoaded('Blizzard_FrameEffects') must \
         return true — auto-discovery picks up the addon (no LoadOnDemand) and \
         `mark_addon_loaded` registers it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_effects_publishes_effect_factory_mixin_with_pool_api(env: &WowLuaEnv) {

    let methods_present: (bool, bool, bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return type(EffectFactoryMixin) == 'table', \
                    type(EffectFactoryMixin.Init) == 'function', \
                    type(EffectFactoryMixin.Attach) == 'function', \
                    type(EffectFactoryMixin.Show) == 'function', \
                    type(EffectFactoryMixin.Hide) == 'function', \
                    type(EffectFactoryMixin.SetShown) == 'function', \
                    type(EffectFactoryMixin.GetExisting) == 'function', \
                    type(EffectFactoryMixin.HasExisting) == 'function'",
        )
        .expect("EffectFactoryMixin probe should succeed");
    assert_eq!(
        methods_present,
        (true, true, true, true, true, true, true, true),
        "EffectFactory.lua:13 publishes `EffectFactoryMixin` with the eight-method \
         framepool-driven effect API: Init (lua:16 — calls CreateFramePool with the \
         derived frameType + template), Attach (lua:21 — base implementation that \
         re-parents + clears anchors + caches originalWidth/originalHeight, derived \
         factories override for direction-specific anchoring), Show (lua:40 — assert \
         animEnum + skip if HasExisting + Acquire from pool + Attach + Show + Play), \
         Hide (lua:56 — StopAnimating + Release back to pool), SetShown (lua:67 — \
         dispatch helper), GetExisting (lua:75 — linear scan over EnumerateActive \
         matching `frame.target`), HasExisting (lua:83 — GetExisting ~= nil)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_effects_publishes_glow_emitter_mixin_with_anim_enum(env: &WowLuaEnv) {

    let mixin_shape: (bool, bool, bool, bool) = env
        .eval(
            "return type(GlowEmitterMixin) == 'table', \
                    type(GlowEmitterMixin.Anims) == 'table', \
                    type(GlowEmitterMixin.OnLoad) == 'function', \
                    type(GlowEmitterMixin.Play) == 'function'",
        )
        .expect("GlowEmitterMixin probe should succeed");
    assert_eq!(
        mixin_shape,
        (true, true, true, true),
        "GlowEmitter.lua:1 publishes `GlowEmitterMixin` with an Anims subtable and \
         OnLoad/Play methods. OnLoad (lua:11) maps Anims enum values to the four \
         AnimationGroup parentKey children (FadeAnim/FaintFadeAnim/\
         NPE_RedButton_GreenGlow/GreenGlow) and sets the NineSlice border blend mode to \
         ADD. Play (lua:22) looks up the AnimationGroup by enum value and calls its \
         Play()"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_effects_glow_emitter_anims_enum_has_four_named_animations(env: &WowLuaEnv) {

    let anim_values: (i32, i32, i32, i32) = env
        .eval(
            "return GlowEmitterMixin.Anims.FadeAnim, \
                    GlowEmitterMixin.Anims.FaintFadeAnim, \
                    GlowEmitterMixin.Anims.NPE_RedButton_GreenGlow, \
                    GlowEmitterMixin.Anims.GreenGlow",
        )
        .expect("GlowEmitterMixin.Anims probe should succeed");
    assert_eq!(
        anim_values,
        (1, 2, 3, 4),
        "GlowEmitter.lua:3-9 publishes the four-value Anims enum: FadeAnim=1, \
         FaintFadeAnim=2, NPE_RedButton_GreenGlow=3, GreenGlow=4. These map to four \
         AnimationGroup parentKey children declared in GlowEmitter.xml:36/43/50/57 — \
         FadeAnim and FaintFadeAnim drive the NineSlice childKey alpha pulse (full \
         intensity 1↔0 vs gentle 1↔0.7), NPE_RedButton_GreenGlow drives the \
         non-smoothed 0.5↔1 pulse on NineSlice (used by the new-player tutorial red \
         button highlight), GreenGlow drives the per-piece (Left/Right/Middle) 0.5↔1 \
         non-smoothed pulse"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_effects_publishes_glow_emitter_factory_derived_from_effect_factory(env: &WowLuaEnv) {

    let factory_shape: (bool, bool, bool, bool, bool) = env
        .eval(
            "return type(GlowEmitterFactory) == 'table', \
                    type(GlowEmitterFactory.Init) == 'function', \
                    type(GlowEmitterFactory.Attach) == 'function', \
                    type(GlowEmitterFactory.Show) == 'function', \
                    type(GlowEmitterFactory.Hide) == 'function'",
        )
        .expect("GlowEmitterFactory probe should succeed");
    assert_eq!(
        factory_shape,
        (true, true, true, true, true),
        "GlowEmitter.lua:28 publishes `GlowEmitterFactory = \
         CreateFromMixins(EffectFactoryMixin)` — the derived factory inherits Init / \
         Show / Hide / SetShown / GetExisting / HasExisting from the parent and \
         OVERRIDES Attach (lua:30) to first call `EffectFactoryMixin.Attach` then add \
         glow-specific anchor logic: when no `width` is supplied, anchor LEFT/RIGHT \
         relative to the target with `offsetX` (default 12) horizontal padding; \
         otherwise anchor CENTER. The override exists so glow effects can stretch to \
         match the target button width by default but auto-center when an explicit \
         width is given"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_effects_glow_emitter_factory_init_runs_at_module_load_creating_pool(env: &WowLuaEnv) {

    let pool_present: (bool, bool) = env
        .eval(
            "return type(GlowEmitterFactory.pool) == 'table', \
                    GlowEmitterFactory ~= EffectFactoryMixin",
        )
        .expect("GlowEmitterFactory pool probe should succeed");
    assert_eq!(
        pool_present,
        (true, true),
        "GlowEmitter.lua:49 calls `GlowEmitterFactory:Init(\"Frame\", \
         \"GlowEmitterTemplate\")` at module-load time — Init (EffectFactory.lua:16) \
         calls `CreateFramePool(frameType, nil, template)` and stashes the pool in \
         `self.pool`, so after the addon loads `GlowEmitterFactory.pool` is a non-nil \
         table. The factory and the parent mixin are distinct tables (CreateFromMixins \
         copies the keys into a fresh table) so callers cannot accidentally mutate the \
         shared EffectFactoryMixin via `GlowEmitterFactory.pool` assignment"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_effects_publishes_glow_emitter_template_via_intrinsic_registry(env: &WowLuaEnv) {

    let template_known: bool = env
        .eval(
            "local f = CreateFrame('Frame', 'WowSimGlowEmitterProbe', UIParent, \
                'GlowEmitterTemplate'); \
             return f ~= nil and type(f.NineSlice) == 'table' \
                 and type(f.Left) == 'table' \
                 and type(f.Right) == 'table' \
                 and type(f.Middle) == 'table'",
        )
        .expect("GlowEmitterTemplate spawn probe should succeed");
    assert!(
        template_known,
        "GlowEmitter.xml:5 declares `<Frame name=\"GlowEmitterTemplate\" \
         mixin=\"GlowEmitterMixin\" virtual=\"true\">` with five parentKey children: \
         a NineSlice (NineSlicePanelTemplate-derived) plus three ARTWORK textures \
         Left / Right / Middle (all with alphaMode=ADD and \
         newplayertutorial-greenglow-redbutton-* atlases) — the Left/Right pieces \
         use useAtlasSize=true and the Middle piece stretches between them. Spawning \
         the template via CreateFrame should yield a frame whose parentKey lookups \
         all resolve to tables"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_effects_publishes_power_swirl_template_via_intrinsic_registry(env: &WowLuaEnv) {

    let template_known: bool = env
        .eval(
            "local f = CreateFrame('Frame', 'WowSimPowerSwirlProbe', UIParent, \
                'PowerSwirlTemplate'); \
             return f ~= nil \
                 and type(f.LightRune) == 'table' \
                 and type(f.BigWhirls) == 'table' \
                 and type(f.SpinningGlows) == 'table' \
                 and type(f.SpinningGlows2) == 'table' \
                 and type(f.RingBurst) == 'table' \
                 and type(f.WhiteStarBurst) == 'table' \
                 and type(f.Ring) == 'table' \
                 and type(f.StarBurst) == 'table' \
                 and type(f.Anim) == 'table'",
        )
        .expect("PowerSwirlTemplate spawn probe should succeed");
    assert!(
        template_known,
        "PowerSwirl.xml:6 declares `<Frame name=\"PowerSwirlTemplate\" virtual=\"true\">` \
         with eight parentKey ARTWORK textures across textureSubLevels -2/6/7 \
         (LightRune at -2, then BigWhirls/SpinningGlows/SpinningGlows2/RingBurst/\
         WhiteStarBurst/Ring at 6, then StarBurst at 7 to render on top) plus an \
         AnimationGroup parentKey=\"Anim\" that drives the per-texture rotate/scale/\
         alpha tweens. SpinningGlows/SpinningGlows2/RingBurst/WhiteStarBurst/StarBurst \
         all inherit the `PowerSwirlScale` virtual texture (scale=1.7079419299744) so \
         they render larger than 1:1. Spawning the template should yield a frame with \
         all eight parentKey textures plus the Anim AnimationGroup resolved"
    );
}
}
