//! Behavior pin for the `ACCOUNT_STORE_TRANSACTION_ERROR` event
//! handling contract in the `Blizzard_AccountStore` lane.
//!
//! Spec/source mismatch finding (PLAN.md task:
//! `EventRegistry:TriggerEvent("ACCOUNT_STORE_TRANSACTION_ERROR")`
//! opens `StaticPopupDialogs["ACCOUNT_STORE_TRANSACTION_ERROR"]` with
//! the localized error text): the plan claims the dispatch path is
//! through `EventRegistry`, but the actual implementation uses a
//! FRAME event on `AccountStoreItemDisplayMixin`. Five separate
//! findings refute and refine the claim:
//!
//! 1. **Wrong event-system.** PLAN says
//!    `EventRegistry:TriggerEvent`. Actual is a FRAME event in
//!    `AccountStoreItemDisplayEvents` at
//!    `Blizzard_AccountStoreItemDisplay.lua:7`, registered via
//!    `FrameUtil.RegisterFrameForEvents(self, AccountStoreItemDisplayEvents)`
//!    at line 67 inside `AccountStoreItemDisplayMixin:OnShow`. Frame
//!    events fire through the `:OnEvent` script-handler path;
//!    EventRegistry callbacks fire through
//!    `EventRegistry:TriggerEvent`. The two systems are separate;
//!    triggering the EventRegistry callback for an
//!    UPPER_SNAKE_CASE name does not dispatch the frame event.
//!
//! 2. **Dispatcher is `AccountStoreItemDisplayMixin:OnEvent`.** The
//!    handler at `Blizzard_AccountStoreItemDisplay.lua:80-98` has
//!    three branches; the third (lines 94-96) reads
//!    `local resultCode = ...` and calls
//!    `StaticPopup_Show("ACCOUNT_STORE_TRANSACTION_ERROR")` — a
//!    constant popup name with NO arguments. The PLAN's "with the
//!    localized error text" framing implies the resultCode flows
//!    into the popup; it does not.
//!
//! 3. **`resultCode` is read and discarded.** The variable is
//!    assigned at line 95 but never used — the call at line 96
//!    passes only the popup name. So firing the event with different
//!    resultCodes (e.g. transient network error vs hard purchase
//!    failure) produces IDENTICAL popup invocations. A regression
//!    that started passing `resultCode` as `text_arg1` to
//!    `StaticPopup_Show` would change the popup text shape (the
//!    popup table at `Blizzard_AccountStoreUtil.lua:11` has no
//!    `text_arg1` substitution, so passing it would be a silent
//!    no-op, but it would change the call signature).
//!
//! 4. **The popup table is constant.**
//!    `StaticPopupDialogs["ACCOUNT_STORE_TRANSACTION_ERROR"]` at
//!    `Blizzard_AccountStoreUtil.lua:11-18` has six fields:
//!    `text` = `NORMAL_FONT_COLOR:WrapTextInColorCode(ACCOUNT_STORE_INCOMPLETE_TRANSACTION)`,
//!    `button1` = `OKAY`,
//!    `showAlert` = `true`,
//!    `hideOnEscape` = `1`,
//!    `timeout` = `0`,
//!    `whileDead` = `1`.
//!    No conditional fields, no OnAccept callback, no data slot —
//!    just an OK-acknowledged alert with a constant message.
//!
//! 5. **The localized text is `ACCOUNT_STORE_INCOMPLETE_TRANSACTION`.**
//!    Resolves to `"The transaction could not be completed."` per
//!    `data/global_strings.rs:23806`. The string is wrapped in
//!    `NORMAL_FONT_COLOR:WrapTextInColorCode(...)` which produces a
//!    `|cAARRGGBB...|r` color-code escape. The popup `text` field is
//!    therefore `"|cFFFFD200The transaction could not be completed.|r"`
//!    (NORMAL_FONT_COLOR being `(1, 0.82, 0)` = `0xFFFFD200` packed).
//!    A regression that swapped to a different `STRING_KEY` (or
//!    dropped the wrap) would change the popup text shape.
//!
//! Five tests pin the contract:
//!
//! - `transaction_error_popup_table_has_expected_six_fields`
//!   asserts the dialog table at
//!   `StaticPopupDialogs["ACCOUNT_STORE_TRANSACTION_ERROR"]` has the
//!   six expected key/value pairs (button1=OKAY, showAlert=true,
//!   hideOnEscape=1, timeout=0, whileDead=1, plus `text` of type
//!   string). Catches additions, removals, and type changes.
//!
//! - `transaction_error_popup_text_contains_localized_string`
//!   asserts the popup `text` field contains the literal substring
//!   `"The transaction could not be completed."`. Pins the
//!   `ACCOUNT_STORE_INCOMPLETE_TRANSACTION` resolution and the wrap
//!   shape (substring match tolerates the color-code prefix/suffix
//!   without hardcoding the color RGB packing).
//!
//! - `event_registry_trigger_does_not_invoke_static_popup_show`
//!   replaces `StaticPopup_Show` with a tracker, fires
//!   `EventRegistry:TriggerEvent("ACCOUNT_STORE_TRANSACTION_ERROR", ...)`,
//!   and asserts the tracker received ZERO calls. PLAN-path tripwire
//!   — flips if a future change wires an EventRegistry callback into
//!   the lane (forcing a re-pin against the new dispatch shape).
//!
//! - `account_store_item_display_on_event_dispatches_constant_popup_name`
//!   replaces `StaticPopup_Show` with a tracker, directly invokes
//!   `AccountStoreItemDisplayMixin.OnEvent(StoreDisplay, "ACCOUNT_STORE_TRANSACTION_ERROR", 7)`,
//!   and asserts (call_count=1, captured_which="ACCOUNT_STORE_TRANSACTION_ERROR",
//!   captured_text_arg1=nil). Pins the dispatch site and the
//!   constant popup name.
//!
//! - `account_store_item_display_on_event_does_not_propagate_result_code`
//!   replaces `StaticPopup_Show` with a tracker, invokes OnEvent
//!   twice with different resultCodes (7 and 999), asserts the two
//!   calls produced IDENTICAL captured args. Pins the
//!   read-and-discard semantic at line 95 — a regression that
//!   started passing resultCode as `text_arg1` would diverge the two
//!   captures.

use crate::common::blizzard_addon_harness::{
    new_blizzard_addon_env, with_blizzard_addon_smoke_shape,
};
use crate::common::load_required_blizzard_addon;
use crate::common::panel_fixtures::{blizzard_ui_dir, load_panel_addons};
use wow_ui_sim::lua_api::WowLuaEnv;

const ROOT: &str = "Blizzard_AccountStore";
const POPUP_KEY: &str = "ACCOUNT_STORE_TRANSACTION_ERROR";
const LOCALIZED_TEXT_SUBSTRING: &str = "The transaction could not be completed.";

#[test]
fn explicit_account_store_load_preserves_popup_after_static_popup_load() {
    let ui_dir = blizzard_ui_dir();
    let env = new_blizzard_addon_env(&ui_dir);

    // AccountStore declares SharedXML, StaticPopup, and UIParentPanelManager.
    // The panel fixture supplies the panel manager's required predecessor chain.
    load_panel_addons(&env);
    load_required_blizzard_addon(&env, &ui_dir, ROOT);
    let popup_registered_before_static_popup_load: bool = env
        .eval(&format!(
            "return C_AddOns.IsAddOnLoaded('Blizzard_AccountStore') and type(StaticPopupDialogs[{POPUP_KEY:?}]) == 'table'"
        ))
        .expect("explicit AccountStore popup probe must run cleanly");
    assert!(
        popup_registered_before_static_popup_load,
        "Explicitly loading `{ROOT}` must register its transaction-error popup before a later \
         `Blizzard_StaticPopup` load."
    );

    env.exec("__account_store_popup_registry_before_reload = StaticPopupDialogs")
        .expect("popup registry identity capture must run cleanly");
    load_required_blizzard_addon(&env, &ui_dir, "Blizzard_StaticPopup");

    let (same_registry, popup_type): (bool, String) = env
        .eval(&format!(
            "return rawequal(__account_store_popup_registry_before_reload, StaticPopupDialogs), type(StaticPopupDialogs[{POPUP_KEY:?}])"
        ))
        .expect("post-StaticPopup AccountStore popup probe must run cleanly");
    assert!(
        same_registry,
        "Loading Blizzard_StaticPopup after `{ROOT}` must not replace its public StaticPopupDialogs registry"
    );
    assert_eq!(
        popup_type, "table",
        "The AccountStore transaction-error popup must survive the later explicit Blizzard_StaticPopup load"
    );
}

#[test]
fn transaction_error_popup_table_has_expected_six_fields() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        let table_type: String = env
            .eval(&format!("return type(StaticPopupDialogs[{POPUP_KEY:?}])"))
            .expect("StaticPopupDialogs probe must run cleanly");

        assert_eq!(
            table_type, "table",
            "Expected `type(StaticPopupDialogs[{POPUP_KEY:?}]) == \"table\"` (declared at \
             `Blizzard_AccountStoreUtil.lua:11-18`), got `{table_type}`. A non-table reading would \
             prove the declaration block did not run (likely a load-order regression where \
             `Blizzard_AccountStoreUtil.lua` was skipped or errored out before the assignment)."
        );

        let text_type: String = env
            .eval(&format!(
                "return type(StaticPopupDialogs[{POPUP_KEY:?}].text)"
            ))
            .expect("popup .text type probe must run cleanly");

        assert_eq!(
            text_type, "string",
            "Expected `type(StaticPopupDialogs[{POPUP_KEY:?}].text) == \"string\"` — the \
             declaration assigns `NORMAL_FONT_COLOR:WrapTextInColorCode(ACCOUNT_STORE_INCOMPLETE_TRANSACTION)`, \
             which `ColorMixin:WrapTextInColorCode` (`SharedColorConstants.lua`) implements as \
             `string.format(\"|cff%s%s|r\", self:GenerateHexColorNoAlpha(), text)` — a string. \
             Got `{text_type}`. A non-string reading would prove either (a) NORMAL_FONT_COLOR is \
             nil at load time (a Blizzard_SharedXML dependency-order regression), or (b) \
             ACCOUNT_STORE_INCOMPLETE_TRANSACTION resolves to a non-string global."
        );

        assert_popup_field_equals_okay(env);
        assert_popup_field_equals_boolean_true(env, "showAlert");
        assert_popup_field_equals_one(env, "hideOnEscape");
        assert_popup_field_equals_zero(env, "timeout");
        assert_popup_field_equals_one(env, "whileDead");
    });
}

fn assert_popup_field_equals_okay(env: &WowLuaEnv) {
    let button1: String = env
        .eval(&format!(
            "return tostring(StaticPopupDialogs[{POPUP_KEY:?}].button1)"
        ))
        .expect("popup .button1 probe must run cleanly");

    let okay_value: String = env
        .eval("return tostring(OKAY)")
        .expect("OKAY global probe must run cleanly");

    assert_eq!(
        button1, okay_value,
        "Expected `StaticPopupDialogs[{POPUP_KEY:?}].button1 == OKAY` (the global at \
         `data/global_strings.rs` resolves to the localized string for \"Okay\"), got \
         `{button1}` vs OKAY=`{okay_value}`. The button1 field controls the single-button \
         acknowledgment text; a regression to `nil` would suppress the OK button entirely."
    );
}

fn assert_popup_field_equals_boolean_true(env: &WowLuaEnv, field_name: &str) {
    let value: bool = env
        .eval(&format!(
            "return StaticPopupDialogs[{POPUP_KEY:?}].{field_name} == true"
        ))
        .expect("popup boolean field probe must run cleanly");

    assert!(
        value,
        "Expected `StaticPopupDialogs[{POPUP_KEY:?}].{field_name} == true` (declared at \
         `Blizzard_AccountStoreUtil.lua:14`). A false reading would prove the alert-styling \
         branch in StaticPopup_Show was bypassed (the popup would render without the alert \
         icon/styling)."
    );
}

fn assert_popup_field_equals_one(env: &WowLuaEnv, field_name: &str) {
    let value: i64 = env
        .eval(&format!(
            "return StaticPopupDialogs[{POPUP_KEY:?}].{field_name}"
        ))
        .expect("popup integer field probe must run cleanly");

    assert_eq!(
        value, 1,
        "Expected `StaticPopupDialogs[{POPUP_KEY:?}].{field_name} == 1` (declared at \
         `Blizzard_AccountStoreUtil.lua:15-17`). A different value would change the lifecycle \
         semantic (hideOnEscape=0 would block Escape dismissal; whileDead=0 would suppress the \
         popup while the player is dead)."
    );
}

fn assert_popup_field_equals_zero(env: &WowLuaEnv, field_name: &str) {
    let value: i64 = env
        .eval(&format!(
            "return StaticPopupDialogs[{POPUP_KEY:?}].{field_name}"
        ))
        .expect("popup integer field probe must run cleanly");

    assert_eq!(
        value, 0,
        "Expected `StaticPopupDialogs[{POPUP_KEY:?}].{field_name} == 0` (declared at \
         `Blizzard_AccountStoreUtil.lua:16` as `timeout = 0`, meaning the popup never \
         auto-dismisses and waits indefinitely for the OK button). A non-zero value would mean \
         the popup auto-dismisses after N seconds — a behavior change that would silently swallow \
         the error if the user wasn't watching."
    );
}

#[test]
fn transaction_error_popup_text_contains_localized_string() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        let popup_text: String = env
            .eval(&format!("return StaticPopupDialogs[{POPUP_KEY:?}].text"))
            .expect("popup .text value probe must run cleanly");

        assert!(
            popup_text.contains(LOCALIZED_TEXT_SUBSTRING),
            "Expected `StaticPopupDialogs[{POPUP_KEY:?}].text` to contain the substring \
             {LOCALIZED_TEXT_SUBSTRING:?} (the resolved value of `ACCOUNT_STORE_INCOMPLETE_TRANSACTION` \
             from `data/global_strings.rs:23806`), got {popup_text:?}. The actual string is \
             wrapped in `|cffXXXXXX...|r` color-code escapes by \
             `NORMAL_FONT_COLOR:WrapTextInColorCode(...)`, so a substring match is the right \
             granularity — it pins the localized message without depending on the exact RGB \
             packing of NORMAL_FONT_COLOR. A miss would prove either (a) the localization key \
             changed (forcing a re-pin against the new STRING_KEY), or (b) the wrap call was \
             dropped, or (c) the text was rebuilt from a different concatenation."
        );
    });
}

#[test]
fn event_registry_trigger_does_not_invoke_static_popup_show() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        seed_static_popup_show_tracker(env);

        env.eval::<()>(&format!(
            r#"
            EventRegistry:TriggerEvent({POPUP_KEY:?}, 7)
            return
            "#
        ))
        .expect("EventRegistry:TriggerEvent must run cleanly");

        let calls: i64 = env
            .eval("return _G.__behavior_transaction_error_show_calls")
            .expect("StaticPopup_Show tracker readout must run cleanly");

        assert_eq!(
            calls, 0,
            "Expected `StaticPopup_Show` to receive ZERO calls after \
             `EventRegistry:TriggerEvent({POPUP_KEY:?}, 7)` (PLAN.md spec/source mismatch \
             tripwire — the plan names EventRegistry as the dispatch path, but the actual event \
             is a FRAME event registered via FrameUtil.RegisterFrameForEvents in \
             AccountStoreItemDisplayMixin:OnShow). Got {calls} call(s). A non-zero reading would \
             prove either (a) some addon registered an EventRegistry callback for the \
             UPPER_SNAKE_CASE event name (unlikely — EventRegistry callbacks follow the \
             dot-namespaced convention `Domain.Event`), or (b) Blizzard added an EventRegistry \
             handler for this event (forcing a re-pin against the new dispatch shape)."
        );

        teardown_static_popup_show_tracker(env);
    });
}

#[test]
fn account_store_item_display_on_event_dispatches_constant_popup_name() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        const SENTINEL_RESULT_CODE: i64 = 7;

        seed_static_popup_show_tracker(env);

        env.eval::<()>(&format!(
            r#"
            local store_display = AccountStoreFrame.StoreDisplay
            AccountStoreItemDisplayMixin.OnEvent(
                store_display,
                {POPUP_KEY:?},
                {SENTINEL_RESULT_CODE}
            )
            return
            "#
        ))
        .expect("Direct OnEvent invocation must run cleanly");

        let (call_count, captured_which, captured_text_arg1_type): (i64, String, String) = env
            .eval(
                "return _G.__behavior_transaction_error_show_calls, \
                 _G.__behavior_transaction_error_captured_which, \
                 _G.__behavior_transaction_error_captured_text_arg1_type",
            )
            .expect("Tracker readout must run cleanly");

        assert_eq!(
            call_count, 1,
            "Expected exactly ONE `StaticPopup_Show` call after a direct invocation of \
             AccountStoreItemDisplayMixin.OnEvent with event=ACCOUNT_STORE_TRANSACTION_ERROR \
             (`Blizzard_AccountStoreItemDisplay.lua:94-96`), got {call_count}. A zero reading \
             would prove the OnEvent body's third branch was bypassed; a value > 1 would prove \
             the branch fan-outs into multiple popups."
        );

        assert_eq!(
            captured_which, POPUP_KEY,
            "Expected the first arg to `StaticPopup_Show` (`which`) to equal {POPUP_KEY:?} (the \
             constant string at `Blizzard_AccountStoreItemDisplay.lua:96`), got \
             {captured_which:?}. A different value would prove the branch dispatched to a \
             different popup template (e.g. a generic error popup, or a per-resultCode popup)."
        );

        assert_eq!(
            captured_text_arg1_type, "nil",
            "Expected the second arg to `StaticPopup_Show` (`text_arg1`) to be nil — the call \
             site passes only the popup name (`Blizzard_AccountStoreItemDisplay.lua:96`: \
             `StaticPopup_Show(\"ACCOUNT_STORE_TRANSACTION_ERROR\")`), got \
             type=`{captured_text_arg1_type}`. A non-nil reading would prove the branch started \
             interpolating the resultCode (or some other context) into the popup text — a \
             behavior change that would alter the popup contract."
        );

        teardown_static_popup_show_tracker(env);
    });
}

#[test]
fn account_store_item_display_on_event_does_not_propagate_result_code() {
    with_blizzard_addon_smoke_shape(&[ROOT], &[], |env, _loaded| {
        const FIRST_RESULT_CODE: i64 = 7;
        const SECOND_RESULT_CODE: i64 = 999;

        seed_static_popup_show_argument_log(env);

        env.eval::<()>(&format!(
            r#"
            local store_display = AccountStoreFrame.StoreDisplay
            AccountStoreItemDisplayMixin.OnEvent(
                store_display, {POPUP_KEY:?}, {FIRST_RESULT_CODE}
            )
            AccountStoreItemDisplayMixin.OnEvent(
                store_display, {POPUP_KEY:?}, {SECOND_RESULT_CODE}
            )
            return
            "#
        ))
        .expect("Two-call OnEvent invocation must run cleanly");

        let (first_call_signature, second_call_signature): (String, String) = env
            .eval(
                "return _G.__behavior_transaction_error_call_log[1], \
                 _G.__behavior_transaction_error_call_log[2]",
            )
            .expect("Argument log readout must run cleanly");

        assert_eq!(
            first_call_signature, second_call_signature,
            "Expected the two `StaticPopup_Show` calls (one per OnEvent invocation, with \
             different resultCodes) to produce IDENTICAL argument signatures — the call site at \
             `Blizzard_AccountStoreItemDisplay.lua:96` reads `local resultCode = ...` at line 95 \
             but discards the variable; only the constant popup name is passed to \
             `StaticPopup_Show`. Got first=`{first_call_signature:?}`, \
             second=`{second_call_signature:?}`. A divergence would prove the resultCode is now \
             being propagated into the popup call (e.g. as text_arg1 or via a new code path), \
             changing the contract."
        );

        teardown_static_popup_show_tracker(env);
    });
}

fn seed_static_popup_show_tracker(env: &WowLuaEnv) {
    env.eval::<()>(
        r#"
        _G.__behavior_transaction_error_show_calls = 0
        _G.__behavior_transaction_error_captured_which = nil
        _G.__behavior_transaction_error_captured_text_arg1_type = "nil"
        _G.__behavior_transaction_error_original_show = StaticPopup_Show
        function StaticPopup_Show(which, text_arg1, _text_arg2, _data, _inserted, _on_hide)
            _G.__behavior_transaction_error_show_calls =
                _G.__behavior_transaction_error_show_calls + 1
            _G.__behavior_transaction_error_captured_which = which
            _G.__behavior_transaction_error_captured_text_arg1_type = type(text_arg1)
            return nil
        end
        return
        "#,
    )
    .expect("seeding StaticPopup_Show tracker must run cleanly");
}

fn seed_static_popup_show_argument_log(env: &WowLuaEnv) {
    env.eval::<()>(
        r#"
        _G.__behavior_transaction_error_call_log = {}
        _G.__behavior_transaction_error_original_show = StaticPopup_Show
        function StaticPopup_Show(which, text_arg1, text_arg2, data)
            local signature = string.format(
                "which=%s|text_arg1=%s|text_arg2=%s|data=%s",
                tostring(which),
                tostring(text_arg1),
                tostring(text_arg2),
                tostring(data)
            )
            table.insert(_G.__behavior_transaction_error_call_log, signature)
            return nil
        end
        return
        "#,
    )
    .expect("seeding StaticPopup_Show argument-log tracker must run cleanly");
}

fn teardown_static_popup_show_tracker(env: &WowLuaEnv) {
    env.eval::<()>(
        r#"
        if _G.__behavior_transaction_error_original_show ~= nil then
            StaticPopup_Show = _G.__behavior_transaction_error_original_show
            _G.__behavior_transaction_error_original_show = nil
        end
        _G.__behavior_transaction_error_show_calls = nil
        _G.__behavior_transaction_error_captured_which = nil
        _G.__behavior_transaction_error_captured_text_arg1_type = nil
        _G.__behavior_transaction_error_call_log = nil
        return
        "#,
    )
    .expect("StaticPopup_Show tracker tear-down must run cleanly");
}
