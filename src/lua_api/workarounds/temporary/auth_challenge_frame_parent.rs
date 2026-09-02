//! Temporary AuthChallengeUI frame-surface repair.
//!
//! AuthChallengeUI loads before the simulator has fully matched Blizzard's
//! secure environment and XML parent-key setup. Keep these repairs isolated
//! until that load path models the frame surface directly.

use crate::lua_api::{LoaderEnv, WowLuaEnv};

const AUTH_CHALLENGE_DEFAULTS_LUA: &str = r#"
C_AuthChallenge = C_AuthChallenge or __wow_namespace()

if rawget(C_AuthChallenge, "SetFrame") == nil then
    function C_AuthChallenge.SetFrame(_frame)
    end
end
if rawget(C_AuthChallenge, "Submit") == nil then
    function C_AuthChallenge.Submit()
    end
end
if rawget(C_AuthChallenge, "Cancel") == nil then
    function C_AuthChallenge.Cancel()
    end
end
if rawget(C_AuthChallenge, "OnTabPressed") == nil then
    function C_AuthChallenge.OnTabPressed(_frame, _reverse)
    end
end
if rawget(C_AuthChallenge, "DidChallengeSucceed") == nil then
    function C_AuthChallenge.DidChallengeSucceed()
        return false
    end
end
"#;

const AUTH_CHALLENGE_FRAME_PARENT_WORKAROUND_LUA: &str = r#"
local authChallengeFunctions = {
    "AuthChallengeUI_OnLoad",
    "AuthChallengeUI_Submit",
    "AuthChallengeUI_Cancel",
    "AuthChallengeUI_OnTabPressed",
    "AuthChallengeUI_OnKeyDown",
}

for _, functionName in ipairs(authChallengeFunctions) do
    if rawget(_G, functionName) == nil
        and type(__secureenv) == "table"
        and type(rawget(__secureenv, functionName)) == "function" then
        rawset(_G, functionName, rawget(__secureenv, functionName))
    end
end

if type(AuthChallengeFrame) ~= "table" or type(UIParent) ~= "table" then
    return
end

if AuthChallengeFrame:GetParent() ~= UIParent then
    AuthChallengeFrame:SetParent(UIParent)
end

local inputFrame = AuthChallengeFrame.InputFrame
if inputFrame and inputFrame.Submit == nil and type(inputFrame.GetChildren) == "function" then
    for _, child in ipairs({ inputFrame:GetChildren() }) do
        if type(child.GetObjectType) == "function"
            and child:GetObjectType() == "Button"
            and type(child.GetText) == "function"
            and (child:GetText() == BLIZZARD_CHALLENGE_SUBMIT or child:GetText() == "Submit") then
            inputFrame.Submit = child
            break
        end
    end
end
"#;

pub(crate) fn apply_bootstrap(lua: &mut rilua::Lua) -> crate::Result<()> {
    lua.exec(AUTH_CHALLENGE_DEFAULTS_LUA)?;
    Ok(())
}

pub(crate) fn patch(env: &LoaderEnv<'_>) {
    let _ = env.exec_public(AUTH_CHALLENGE_FRAME_PARENT_WORKAROUND_LUA);
}

pub(crate) fn patch_env(env: &WowLuaEnv) {
    let _ = env.exec(AUTH_CHALLENGE_FRAME_PARENT_WORKAROUND_LUA);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_challenge_defaults_to_unsuccessful_noops() {
        let env = WowLuaEnv::new().expect("lua env should initialize");
        let succeeded: bool = env
            .eval(
                r#"
                C_AuthChallenge.SetFrame({})
                C_AuthChallenge.Submit()
                C_AuthChallenge.Cancel()
                C_AuthChallenge.OnTabPressed(nil, false)
                return C_AuthChallenge.DidChallengeSucceed()
                "#,
            )
            .expect("auth challenge defaults should be callable");

        assert!(!succeeded);
    }

    #[test]
    fn preserves_existing_auth_challenge_functions() {
        let env = WowLuaEnv::new().expect("lua env should initialize");
        env.exec(
            r#"
            function C_AuthChallenge.DidChallengeSucceed()
                return true
            end
            "#,
        )
        .expect("fixture should install existing function");

        apply_bootstrap(&mut env.rilua_mut()).expect("workaround should apply");

        let succeeded: bool = env
            .eval("return C_AuthChallenge.DidChallengeSucceed()")
            .expect("existing auth challenge function should remain callable");

        assert!(succeeded);
    }

    #[test]
    fn restores_missing_auth_challenge_functions_from_secureenv() {
        let env = WowLuaEnv::new().expect("lua env should initialize");
        env.exec(
            r#"
            __secureenv = {
                AuthChallengeUI_OnLoad = function()
                    return "secure-onload"
                end,
                AuthChallengeUI_Submit = function()
                    return "secure-submit"
                end,
            }
            "#,
        )
        .expect("secure env should install");

        patch_env(&env);

        let (on_load, submit): (String, String) = env
            .eval("return AuthChallengeUI_OnLoad(), AuthChallengeUI_Submit()")
            .expect("restored auth challenge functions should be readable");

        assert_eq!(on_load, "secure-onload");
        assert_eq!(submit, "secure-submit");
    }

    #[test]
    fn reparents_auth_challenge_frame_to_uiparent() {
        let env = WowLuaEnv::new().expect("lua env should initialize");
        env.exec(
            r#"
            UIParent = {}
            local oldParent = {}
            AuthChallengeFrame = {
                parent = oldParent,
                GetParent = function(self)
                    return self.parent
                end,
                SetParent = function(self, parent)
                    self.parent = parent
                end,
            }
            "#,
        )
        .expect("auth challenge frame should install");

        patch_env(&env);

        let has_ui_parent: bool = env
            .eval("return AuthChallengeFrame:GetParent() == UIParent")
            .expect("auth challenge parent should be readable");

        assert!(has_ui_parent);
    }

    #[test]
    fn repairs_input_frame_submit_button_from_children() {
        let env = WowLuaEnv::new().expect("lua env should initialize");
        env.exec(
            r#"
            BLIZZARD_CHALLENGE_SUBMIT = "Submit"
            UIParent = {}
            local cancel = {
                GetObjectType = function()
                    return "Button"
                end,
                GetText = function()
                    return "Cancel"
                end,
            }
            submitButton = {
                GetObjectType = function()
                    return "Button"
                end,
                GetText = function()
                    return "Submit"
                end,
            }
            AuthChallengeFrame = {
                parent = UIParent,
                GetParent = function(self)
                    return self.parent
                end,
                SetParent = function(self, parent)
                    self.parent = parent
                end,
                InputFrame = {
                    GetChildren = function()
                        return cancel, submitButton
                    end,
                },
            }
            "#,
        )
        .expect("auth challenge input frame should install");

        patch_env(&env);

        let repaired_submit: bool = env
            .eval("return AuthChallengeFrame.InputFrame.Submit == submitButton")
            .expect("auth challenge submit button should be readable");

        assert!(repaired_submit);
    }
}
