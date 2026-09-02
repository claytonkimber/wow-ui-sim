//! Startup publication and values for 12.0.0 CAA constants.

use super::super::*;

#[cfg(feature = "retail-12-0-0")]
#[test]
fn test_patch_12_0_0_caa_constants_publish_exact_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
            if type(Constants) ~= "table" then return "Constants:type" end
            if type(Constants.CAAConstants) ~= "table" then return "CAAConstants:type" end

            local expected = {
                CAAEnabledDefault = false,
                CAAFrequencyDefault = 0,
                CAAFrequencyMax = 10,
                CAAFrequencyMin = -10,
                CAAInterruptCastDefault = 0,
                CAAInterruptCastSuccessDefault = 0,
                CAAMinCastTimeDefault = 1.5,
                CAAMinCastTimeMax = 5,
                CAAMinCastTimeMin = 0,
                CAAMinCastTimeStep = 0.5,
                CAAPartyHealthPercentDefault = 0,
                CAAPlayerCastFormatDefault = 4,
                CAAPlayerCastModeDefault = 0,
                CAAPlayerHealthFormatDefault = 1,
                CAAPlayerHealthPercentDefault = 0,
                CAAPlayerResourceFormatDefault = 1,
                CAAPlayerResourcePercentDefault = 0,
                CAASampleTextThrottleTime = 1,
                CAASayCombatEndDefault = true,
                CAASayCombatStartDefault = true,
                CAASayIfTargetedDefault = 1,
                CAATargetCastFormatDefault = 0,
                CAATargetCastModeDefault = 0,
                CAATargetDeathBehaviorDefault = 0,
                CAATargetHealthFormatDefault = 3,
                CAATargetHealthPercentDefault = 2,
                CAATargetNameDefault = true,
                CAAThrottleDefault = 0,
                CAAThrottleMax = 5,
                CAAThrottleMin = 0,
                CAAThrottleStep = 0.5,
                CAAVoiceDefault = 0,
            }
            for name, value in pairs(expected) do
                local actual = Constants.CAAConstants[name]
                if type(actual) ~= type(value) then
                    return name .. ":type=" .. type(actual)
                end
                if actual ~= value then
                    return name .. ":value"
                end
            end
            return "ok"
            "#,
        )
        .unwrap();

    assert_eq!(result, "ok");
}
