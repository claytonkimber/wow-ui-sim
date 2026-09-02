//! Force-load checkbox behavior for `Blizzard_AddOnList`.

use crate::common::blizzard_addon_harness::with_blizzard_addon_startup_shape;

const ROOT: &str = "Blizzard_AddOnList";

#[test]
fn force_load_click_toggles_addon_version_check() {
    with_blizzard_addon_startup_shape(&[ROOT], &[], |env, _loaded| {
        let (enabled_after_click_to_checked, enabled_after_click_to_unchecked): (bool, bool) = env
            .eval(
                r#"
                C_AddOns.SetAddonVersionCheck(true)
                AddonList.ForceLoad:SetChecked(false)
                AddonList.ForceLoad:Click()
                local enabledAfterClickToChecked = C_AddOns.IsAddonVersionCheckEnabled()

                AddonList.ForceLoad:SetChecked(true)
                AddonList.ForceLoad:Click()
                local enabledAfterClickToUnchecked = C_AddOns.IsAddonVersionCheckEnabled()

                return enabledAfterClickToChecked, enabledAfterClickToUnchecked
                "#,
            )
            .expect("AddonList ForceLoad click probe must run cleanly");

        assert!(
            !enabled_after_click_to_checked,
            "Clicking `AddonList.ForceLoad` to checked must disable addon version checking"
        );
        assert!(
            enabled_after_click_to_unchecked,
            "Clicking `AddonList.ForceLoad` to unchecked must enable addon version checking"
        );
    });
}
