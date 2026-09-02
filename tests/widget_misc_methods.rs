//! Tests for widget_misc methods: SetupMenu, SetAlertContainer, mixin delegation, etc.

use wow_ui_sim::lua_api::WowLuaEnv;

#[test]
fn test_frame_debug_env_does_not_change_table_length() {
    let env = WowLuaEnv::new().unwrap();

    let result: (f64, bool, String) = env
        .eval(
            r#"
            local frame = CreateFrame("Frame", "TestFrameDebugEnvLength", UIParent)
            frame:SetupMenu(function() end)
            local fields = debug.getfenv(frame)[1]
            return #frame, type(fields) == "table", tostring(rawget(frame, 1))
        "#,
        )
        .unwrap();

    assert_eq!(
        result.0, 0.0,
        "hidden frame fields should not occupy the frame array part: raw slot 1 was {}",
        result.2
    );
    assert!(result.1, "debug.getfenv(frame)[1] should expose fields");
}

#[test]
fn test_frame_debug_env_exposes_live_newindex_fields() {
    let env = WowLuaEnv::new().unwrap();

    let result: (String, f64, String, f64, f64) = env
        .eval(
            r#"
            local frame = CreateFrame("Frame", "TestFrameDebugEnvSnapshot", UIParent)
            frame.beforeEnv = "before"
            frame.count = 1
            local fields = debug.getfenv(frame)[1]
            frame.afterEnv = "after"
            frame.count = 2
            return fields.beforeEnv, fields.count, fields.afterEnv, frame.count, rawget(frame, "count")
        "#,
        )
        .unwrap();

    assert_eq!(result.0, "before");
    assert_eq!(result.1, 2.0);
    assert_eq!(result.2, "after");
    assert_eq!(result.3, 2.0);
    assert_eq!(result.4, 2.0);
}

#[test]
fn test_widget_misc_group_timer_method_surface_is_registered() {
    let env = WowLuaEnv::new().unwrap();
    let missing_method: Option<String> = env
        .eval(
            r#"
            local frame = CreateFrame("Frame", "TestWidgetMiscGroupTimerSurfaceFrame", UIParent)
            local methods = {
                "GetOrCreateGroup",
                "ForceUpdateTimers",
                "RegisterFontString",
                "RegisterFontStrings",
                "RegisterBackgroundTexture",
                "RegisterFrames",
                "SetBorderAlpha",
                "SetBorderScalar",
                "SetBorderTexture",
                "SetFillAlpha",
                "SetOwningDialog",
                "SetFillTexture",
                "DrawBlob",
                "SetToDefaults",
                "DrawNone",
                "UpdateMouseOverTooltip",
                "GetTooltipIndex",
                "SetAlertContainer",
                "SetDefaultText",
                "UpdateHeight",
                "SetSelectionTranslator",
                "SetItemButtonScale",
                "UpdateItemContextMatching",
                "RegisterForWidgetSet",
                "UnregisterForWidgetSet",
            }

            for _, method in ipairs(methods) do
                if type(frame[method]) ~= "function" then
                    return method
                end
            end

            return nil
        "#,
        )
        .unwrap();

    assert!(
        missing_method.is_none(),
        "missing widget misc method: {missing_method:?}"
    );
}

#[test]
fn test_widget_misc_setup_and_alert_methods_persist_runtime_fields() {
    let env = WowLuaEnv::new().unwrap();

    let result: (bool, bool, bool) = env
        .eval(
            r#"
            local frame = CreateFrame("Frame", "TestWidgetMiscFrame", UIParent)
            local fallbackGenerator = function() end
            frame:SetupMenu(fallbackGenerator)
            local fields = debug.getfenv(frame)[1]
            local fallbackStored = fields.menuGenerator == fallbackGenerator

            local overrideFrame = CreateFrame("Frame", "TestWidgetMiscOverrideFrame", UIParent)
            local overrideFields = debug.getfenv(overrideFrame)[1]
            local overrideGenerator = function() end
            overrideFields.SetupMenu = function(self, generator)
                rawset(overrideFields, "calledGenerator", generator)
            end
            overrideFrame:SetupMenu(overrideGenerator)
            local overrideCalled = overrideFields.calledGenerator == overrideGenerator

            local container = CreateFrame("Frame", "TestWidgetMiscContainer", UIParent)
            frame:SetAlertContainer(container)
            local alertStored = fields.alertContainer == container

            return fallbackStored, overrideCalled, alertStored
        "#,
        )
        .unwrap();

    assert!(
        result.0,
        "SetupMenu should store menuGenerator without an override"
    );
    assert!(
        result.1,
        "SetupMenu should delegate to an existing mixin override instead of shadowing it"
    );
    assert!(
        result.2,
        "SetAlertContainer should store the container reference on the frame"
    );
}

#[test]
fn test_widget_misc_is_menu_open_uses_dropdown_menu_state() {
    let env = WowLuaEnv::new().unwrap();

    let result: (bool, bool) = env
        .eval(
            r#"
            local frame = CreateFrame("Frame", "TestWidgetMiscMenuFrame", UIParent)
            local fields = debug.getfenv(frame)[1]
            local closed = frame:IsMenuOpen() == false

            fields.menu = {}
            local opened = frame:IsMenuOpen() == true

            return closed, opened
        "#,
        )
        .unwrap();

    assert!(result.0, "Frames should report no menu by default");
    assert!(
        result.1,
        "Frames should report an open menu when their active menu field is set"
    );
}

#[test]
fn test_widget_misc_set_owning_dialog_delegates_or_stores_state() {
    let env = WowLuaEnv::new().unwrap();

    let result: (bool, bool) = env
        .eval(
            r#"
            local frame = CreateFrame("Frame", "TestWidgetMiscOwningDialogFrame", UIParent)
            local dialog = CreateFrame("Frame", "TestWidgetMiscOwningDialogDialog", UIParent)
            local fields = debug.getfenv(frame)[1]
            frame:SetOwningDialog(dialog)
            local stored = fields.owningDialog == dialog

            local overrideFrame = CreateFrame("Frame", "TestWidgetMiscOwningDialogOverrideFrame", UIParent)
            local overrideDialog = CreateFrame("Frame", "TestWidgetMiscOwningDialogOverrideDialog", UIParent)
            local overrideFields = debug.getfenv(overrideFrame)[1]
            overrideFields.SetOwningDialog = function(self, value)
                rawset(overrideFields, "storedDialog", value)
            end
            overrideFrame:SetOwningDialog(overrideDialog)
            local delegated = overrideFields.storedDialog == overrideDialog

            return stored, delegated
        "#,
        )
        .unwrap();

    assert!(
        result.0,
        "SetOwningDialog should store the owning dialog on the frame"
    );
    assert!(
        result.1,
        "SetOwningDialog should delegate to an existing mixin override instead of shadowing it"
    );
}

#[test]
fn test_widget_misc_registration_methods_delegate_or_store_state() {
    let env = WowLuaEnv::new().unwrap();

    let result: (bool, bool, bool, bool) = env
        .eval(
            r##"
            local frame = CreateFrame("Frame", "TestWidgetMiscRegistrationFrame", UIParent)
            local fontA = frame:CreateFontString(nil, "OVERLAY")
            local fontB = frame:CreateFontString(nil, "OVERLAY")
            local childA = CreateFrame("Frame", nil, frame)
            local childB = CreateFrame("Frame", nil, frame)
            local background = frame:CreateTexture(nil, "BACKGROUND")
            local fields = debug.getfenv(frame)[1]

            frame:RegisterFontStrings(fontA, fontB)
            frame:RegisterFrames(childA, childB)
            frame:RegisterBackgroundTexture(background, "guildrename")

            local fontStringsStored = fields.fontStrings and fields.fontStrings[1] == fontA and fields.fontStrings[2] == fontB
            local framesStored = fields.frames and fields.frames[1] == childA and fields.frames[2] == childB
            local backgroundStored = fields.backgroundTexture == background and fields.textureKit == "guildrename"

            local overrideFrame = CreateFrame("Frame", "TestWidgetMiscRegistrationOverrideFrame", UIParent)
            local overrideFont = overrideFrame:CreateFontString(nil, "OVERLAY")
            local overrideFields = debug.getfenv(overrideFrame)[1]
            overrideFields.RegisterFontStrings = function(self, ...)
                rawset(overrideFields, "storedCount", select("#", ...))
                rawset(overrideFields, "storedFirst", ...)
            end
            overrideFrame:RegisterFontStrings(overrideFont)
            local delegated = overrideFields.storedCount == 1 and overrideFields.storedFirst == overrideFont

            return fontStringsStored, framesStored, backgroundStored, delegated
        "##,
        )
        .unwrap();

    assert!(
        result.0,
        "RegisterFontStrings should store the registered font strings on the frame"
    );
    assert!(
        result.1,
        "RegisterFrames should store the registered frames on the frame"
    );
    assert!(
        result.2,
        "RegisterBackgroundTexture should store the background texture and texture kit"
    );
    assert!(
        result.3,
        "RegisterFontStrings should delegate to an existing mixin override instead of shadowing it"
    );
}

#[test]
fn test_widget_misc_register_font_string_delegates_or_adds_to_set() {
    let env = WowLuaEnv::new().unwrap();

    let result: (bool, bool, bool) = env
        .eval(
            r##"
            local frame = CreateFrame("Frame", "TestWidgetMiscRegisterFontStringFrame", UIParent)
            local fontA = frame:CreateFontString(nil, "OVERLAY")
            local fontB = frame:CreateFontString(nil, "OVERLAY")
            local fields = debug.getfenv(frame)[1]
            fields.fontStrings = {}

            frame:RegisterFontString(fontA)
            frame:RegisterFontString(fontB)
            local registered = fields.fontStrings[fontA] == true and fields.fontStrings[fontB] == true

            local overrideFrame = CreateFrame("Frame", "TestWidgetMiscRegisterFontStringOverrideFrame", UIParent)
            local overrideFont = overrideFrame:CreateFontString(nil, "OVERLAY")
            local overrideFields = debug.getfenv(overrideFrame)[1]
            overrideFields.RegisterFontString = function(self, fontString)
                rawset(overrideFields, "storedSelf", self)
                rawset(overrideFields, "storedFontString", fontString)
            end
            overrideFrame:RegisterFontString(overrideFont)
            local delegated = overrideFields.storedSelf == overrideFrame and overrideFields.storedFontString == overrideFont

            return registered, fields.__registeredFontStrings == fields.fontStrings, delegated
        "##,
        )
        .unwrap();

    assert!(
        result.0,
        "RegisterFontString should add each font string to the frame's fontStrings set"
    );
    assert!(
        result.1,
        "RegisterFontString should expose the same registry through __registeredFontStrings"
    );
    assert!(
        result.2,
        "RegisterFontString should delegate to an existing mixin override instead of shadowing it"
    );
}

#[test]
fn test_widget_misc_item_button_methods_delegate_or_store_state() {
    let env = WowLuaEnv::new().unwrap();

    let result: (bool, bool, bool, bool, bool) = env
        .eval(
            r#"
            local frame = CreateFrame("Frame", "TestWidgetMiscItemFrame", UIParent)
            local fields = debug.getfenv(frame)[1]
            local translator = function(selection) return selection end
            frame:SetSelectionTranslator(translator)
            local translatorStored = fields.selectionTranslator == translator

            frame:SetItemButtonScale(0.65)
            local scaleStored = fields.itemButtonScale == 0.65

            local overrideFrame = CreateFrame("Frame", "TestWidgetMiscItemOverrideFrame", UIParent)
            local overrideFields = debug.getfenv(overrideFrame)[1]
            local overrideTranslator = function(selection) return selection.data end
            overrideFields.SetSelectionTranslator = function(self, value)
                rawset(overrideFields, "storedTranslator", value)
            end
            overrideFields.SetItemButtonScale = function(self, value)
                rawset(overrideFields, "storedScale", value)
            end
            overrideFields.UpdateItemContextMatching = function(self)
                rawset(overrideFields, "updateCalled", true)
            end

            overrideFrame:SetSelectionTranslator(overrideTranslator)
            overrideFrame:SetItemButtonScale(1.4)
            overrideFrame:UpdateItemContextMatching()

            return translatorStored,
                scaleStored,
                overrideFields.storedTranslator == overrideTranslator,
                overrideFields.storedScale == 1.4,
                overrideFields.updateCalled == true
        "#,
        )
        .unwrap();

    assert!(
        result.0,
        "SetSelectionTranslator should store the fallback translator on the frame"
    );
    assert!(
        result.1,
        "SetItemButtonScale should store the fallback scale on the frame"
    );
    assert!(
        result.2,
        "SetSelectionTranslator should delegate to an existing mixin override"
    );
    assert!(
        result.3,
        "SetItemButtonScale should delegate to an existing mixin override"
    );
    assert!(
        result.4,
        "UpdateItemContextMatching should delegate to an existing mixin override"
    );
}

#[test]
fn test_widget_misc_visual_methods_delegate_or_store_state() {
    let env = WowLuaEnv::new().unwrap();

    let result: (bool, bool, bool, bool) = env
        .eval(
            r##"
            local frame = CreateFrame("Frame", "TestWidgetMiscVisualFrame", UIParent)
            local frameFields = debug.getfenv(frame)[1]
            frame:SetDefaultText("fallback")
            local defaultStored = frameFields.defaultText == "fallback"

            local texture = frame:CreateTexture(nil, "ARTWORK")
            local textureFields = debug.getfenv(texture)[1]
            texture:SetVisuals("left", 42, true)
            local visualStored = textureFields.visualArgs
                and textureFields.visualArgs[1] == "left"
                and textureFields.visualArgs[2] == 42
                and textureFields.visualArgs[3] == true

            local overrideFrame = CreateFrame("Frame", "TestWidgetMiscVisualOverrideFrame", UIParent)
            local overrideFrameFields = debug.getfenv(overrideFrame)[1]
            overrideFrameFields.SetDefaultText = function(self, text)
                rawset(overrideFrameFields, "storedDefaultText", text)
            end
            overrideFrameFields.UpdateHeight = function(self)
                rawset(overrideFrameFields, "heightUpdated", true)
            end

            local overrideTexture = overrideFrame:CreateTexture(nil, "ARTWORK")
            local overrideTextureFields = debug.getfenv(overrideTexture)[1]
            overrideTextureFields.SetVisuals = function(self, ...)
                rawset(overrideTextureFields, "visualCount", select("#", ...))
                rawset(overrideTextureFields, "visualFirst", (...))
            end

            overrideFrame:SetDefaultText("override")
            overrideFrame:UpdateHeight()
            overrideTexture:SetVisuals("tierSlot", 7, false)

            return defaultStored,
                visualStored,
                overrideFrameFields.storedDefaultText == "override"
                    and overrideTextureFields.visualCount == 3
                    and overrideTextureFields.visualFirst == "tierSlot",
                overrideFrameFields.heightUpdated == true
        "##,
        )
        .unwrap();

    assert!(
        result.0,
        "SetDefaultText should store fallback default text on the frame"
    );
    assert!(
        result.1,
        "SetVisuals should store fallback visual arguments on the frame"
    );
    assert!(
        result.2,
        "SetDefaultText and SetVisuals should delegate to existing mixin overrides"
    );
    assert!(
        result.3,
        "UpdateHeight should delegate to an existing mixin override"
    );
}

#[test]
fn test_widget_misc_widget_set_methods_delegate_or_store_registration() {
    let env = WowLuaEnv::new().unwrap();

    let result: (bool, bool, bool) = env
        .eval(
            r##"
            local frame = CreateFrame("Frame", "TestWidgetSetFrame", UIParent)
            local fields = debug.getfenv(frame)[1]
            local layout = function() end
            local init = function() end

            frame:RegisterForWidgetSet(5501, layout, init, "player")
            local fallbackStored = fields.widgetSetRegistration
                and fields.widgetSetRegistration.widgetSetID == 5501
                and fields.widgetSetRegistration.widgetLayoutFunction == layout
                and fields.widgetSetRegistration.widgetInitFunction == init
                and fields.widgetSetRegistration.attachedUnitInfo == "player"

            frame:UnregisterForWidgetSet()
            local fallbackCleared = fields.widgetSetRegistration == nil

            local overrideFrame = CreateFrame("Frame", "TestWidgetSetOverrideFrame", UIParent)
            local overrideFields = debug.getfenv(overrideFrame)[1]
            overrideFields.RegisterForWidgetSet = function(self, ...)
                rawset(overrideFields, "registerCount", select("#", ...))
                rawset(overrideFields, "registeredWidgetSetID", ...)
            end
            overrideFields.UnregisterForWidgetSet = function(self, ...)
                rawset(overrideFields, "unregisterCount", select("#", ...))
                rawset(overrideFields, "unregisteredWidgetSetID", ...)
            end

            overrideFrame:RegisterForWidgetSet(4402, layout, init, "target")
            overrideFrame:UnregisterForWidgetSet(4402)

            return fallbackStored == true,
                fallbackCleared == true,
                overrideFields.registerCount == 4
                    and overrideFields.registeredWidgetSetID == 4402
                    and overrideFields.unregisterCount == 1
                    and overrideFields.unregisteredWidgetSetID == 4402
        "##,
        )
        .unwrap();

    assert!(
        result.0,
        "RegisterForWidgetSet should store fallback registration data on the frame"
    );
    assert!(
        result.1,
        "UnregisterForWidgetSet should clear the fallback widget set registration"
    );
    assert!(
        result.2,
        "Widget set methods should delegate to existing mixin overrides"
    );
}
