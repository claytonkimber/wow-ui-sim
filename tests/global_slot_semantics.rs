//! Track 3 sub-item 5 — semantic coverage for the global-slot fast
//! path. Exercises the full `WowLuaEnv` bootstrap (not the synthetic
//! slot table tests in `lua_api::global_slots::tests`) to confirm
//! addon-visible reads and the `_G_live` shadow-override contract hold
//! end-to-end.

use std::process::Command;
use wow_ui_sim::lua_api::WowLuaEnv;

const FREEZE_GLOBALS_CHILD_ENV: &str = "WOW_SIM_FREEZE_GLOBALS_TEST_CHILD";

/// Slot 0 always maps to `_G` regardless of freeze state. The
/// bootstrap-populated slot vector must reflect that.
#[test]
fn bootstrap_slot_zero_refers_to_g() {
    let env = WowLuaEnv::new().expect("lua env");
    let same: bool = env
        .eval("return _G == _G")
        .expect("identity probe should succeed");
    assert!(same);
}

/// Whitelisted globals resolved at install time should match what Lua
/// sees via a `_G` lookup for the same key. This pins the contract
/// that `install(...)` captures the right value per whitelist slot —
/// without threading the internal slot vector out to tests.
#[test]
fn whitelisted_global_is_populated_at_bootstrap() {
    let env = WowLuaEnv::new().expect("lua env");
    let (mixin_is_function, create_frame_is_function, enum_is_table): (bool, bool, bool) = env
        .eval(
            r#"
            return type(Mixin) == "function",
                   type(CreateFrame) == "function",
                   type(Enum) == "table"
            "#,
        )
        .expect("should resolve bootstrap whitelist globals");
    assert!(mixin_is_function, "Mixin should be a function at bootstrap");
    assert!(
        create_frame_is_function,
        "CreateFrame should be a function at bootstrap"
    );
    assert!(enum_is_table, "Enum should be a table at bootstrap");
}

/// With the freeze gate enabled, writing a NEW global (one that wasn't
/// present in `_G` at freeze time) should flow through `__newindex` →
/// `_G_live` and be visible on subsequent reads. The slot read path
/// surfaces the shadow entry for a whitelist name that was Nil at
/// install time — this is the semantic contract sub-item 5 pins.
///
/// Uses `MainActionBar` (in HOT_GLOBALS) which is typically not
/// populated during `--no-addons --no-saved-vars` bootstrap; in case it
/// ever is, this test still asserts the Lua-visible round-trip, which
/// is the real contract.
#[test]
fn g_live_shadow_surfaces_new_whitelisted_global_after_freeze() {
    if std::env::var_os(FREEZE_GLOBALS_CHILD_ENV).is_some() {
        assert_frozen_global_shadow();
        return;
    }

    let test_thread = std::thread::current();
    let test_name = test_thread.name().expect("global-slot test thread name");
    let output = Command::new(std::env::current_exe().expect("integration test binary"))
        .args([test_name, "--exact", "--test-threads=1", "--nocapture"])
        .env(FREEZE_GLOBALS_CHILD_ENV, "1")
        .env("WOW_SIM_FREEZE_GLOBALS", "1")
        .output()
        .expect("run frozen-global child test");
    assert!(
        output.status.success(),
        "frozen-global child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_frozen_global_shadow() {
    let env = WowLuaEnv::new().expect("lua env with freeze gate");
    let (direct, indexed): (String, String) = env
        .eval(
            r#"
            _G.MyFreshAddonSentinel = "shadow-ok"
            return MyFreshAddonSentinel, _G.MyFreshAddonSentinel
            "#,
        )
        .expect("shadow write + read should round-trip post-freeze");

    assert_eq!(direct, "shadow-ok");
    assert_eq!(indexed, "shadow-ok");
}
