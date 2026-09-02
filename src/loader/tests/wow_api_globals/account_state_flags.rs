//! Startup publication and values for the 12.0.0 account-state enum families.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

const ACCOUNT_STATE_LOADED_FLAGS: &[(&str, &str)] = &[
    ("AccountCurrenciesLoaded", "0x0000000000200000"),
    ("AccountFactionsLoaded", "0x0000000200000000"),
    ("AccountItemsLoaded", "0x0000000400000000"),
    ("AccountMappingLoaded", "0x0000008000000000"),
    ("AccountNotificationsLoaded", "0x0000000008000000"),
    ("AccountWowlabsLoaded", "0x0000000020000000"),
    ("AchievementsLoaded", "0x0000000000000001"),
    ("ArchivedPurchasesLoaded", "0x0000000000000200"),
    ("AuctionableTokensLoaded", "0x0000000000002000"),
    ("BanktabSettingsLoaded", "0x0000004000000000"),
    ("BattleNetAccountLoaded", "0x0000000000100000"),
    ("BitVectorsLoaded", "0x0000000100000000"),
    ("BpayAddLicenseObjectsLoaded", "0x0000000000000800"),
    ("BpayDistributionObjectsLoaded", "0x0000000000000100"),
    ("BpayProductitemObjectsLoaded", "0x0000000000020000"),
    ("CharacterItemsLoaded", "0x0000010000000000"),
    ("CharactersLoaded", "0x0000000000000040"),
    ("CombinedQuestLogLoaded", "0x0000000800000000"),
    ("ConsumableTokensLoaded", "0x0000000000004000"),
    ("CriteriaLoaded", "0x0000000000000002"),
    ("CurrencyCapsLoaded", "0x0000000000000010"),
    ("CurrencyTransferLogLoaded", "0x0000020000000000"),
    ("DataElementsLoaded", "0x0000001000000000"),
    ("DynamicCriteriaLoaded", "0x0000000001000000"),
    ("EventRecordsLoaded", "0x0000200000000000"),
    ("HousingDataLoaded", "0x0000080000000000"),
    ("ItemCollectionsLoaded", "0x0000000000001000"),
    ("LgVendorPurchaseLoaded", "0x0000040000000000"),
    ("LoadedNone", "0x0000000000000000"),
    ("MountsLoaded", "0x0000000000000004"),
    ("PerksHeldItemLoaded", "0x0000000040000000"),
    ("PerksPastRewardsLoaded", "0x0000000000008000"),
    ("PerksPendingPurchaseLoaded", "0x0000000010000000"),
    ("PerksPendingRewardsLoaded", "0x0000000080000000"),
    ("PetjournalInitialized", "0x0000000000000008"),
    ("PurchasesLoaded", "0x0000000000000080"),
    ("QuestCriteriaLoaded", "0x0000000000080000"),
    ("QuestLogLoaded", "0x0000000000000020"),
    ("RafActivityLoaded", "0x0000000002000000"),
    ("RafBalanceLoaded", "0x0000000000400000"),
    ("RafRewardsLoaded", "0x0000000000800000"),
    ("RevokedRafRewardsLoaded", "0x0000000004000000"),
    ("SettingsLoaded", "0x0000000000000400"),
    ("TransmogOutfitsLoaded", "0x0000400000000000"),
    ("TrialBoostHistoryLoaded", "0x0000000000040000"),
    ("VasTransactionsLoaded", "0x0000000000010000"),
    ("WarbandScenesLoaded", "0x0000100000000000"),
    ("WarbandsLoaded", "0x0000002000000000"),
];

const CREATE_ALL_ACCOUNT_DATA: &[(&str, &str)] = &[
    ("AccountCurrenciesDone", "0x0000000000200000"),
    ("AccountDynamicCriteriaDone", "0x0000000001000000"),
    ("AccountFactionsDone", "0x0000000200000000"),
    ("AccountItemsDone", "0x0000000400000000"),
    ("AccountMappingDone", "0x0000008000000000"),
    ("AccountNotificationsDone", "0x0000000008000000"),
    ("AccountStateHousingData", "0x0000080000000000"),
    ("AchievementsDone", "0x0000000000000001"),
    ("ArchivedPurchasesDone", "0x0000000000000200"),
    ("AuctionableTokensDone", "0x0000000000002000"),
    ("BanktabSettingsDone", "0x0000004000000000"),
    ("BattlepetsDone", "0x0000000000000008"),
    ("BitVectorsDone", "0x0000000100000000"),
    ("BpayAddLicenseObjectsDone", "0x0000000000000800"),
    ("BpayDistributionObjectsDone", "0x0000000000000100"),
    ("BpayProductitemObjectsDone", "0x0000000000020000"),
    ("CharacterItemsDone", "0x0000010000000000"),
    ("CharactersDone", "0x0000000000000040"),
    ("CombinedQuestLogEntriesDone", "0x0000000800000000"),
    ("ConsumableTokensDone", "0x0000000000004000"),
    ("CriteriaDone", "0x0000000000000002"),
    ("CurrencyTransferLogDone", "0x0000020000000000"),
    ("CurrencycapsDone", "0x0000000000000010"),
    ("DataElementsDone", "0x0000001000000000"),
    ("EventRecordsDone", "0x0000200000000000"),
    ("ItemCollectionItemsDone", "0x0000000000001000"),
    ("LgVendorPurchaseDone", "0x0000040000000000"),
    ("MountsDone", "0x0000000000000004"),
    ("None", "0x0000000000000000"),
    ("Object", "0x0000000000100000"),
    ("PerkHeldItemsDone", "0x0000000040000000"),
    ("PerkPastRewardsDone", "0x0000000000008000"),
    ("PerkPendingPurchasesDone", "0x0000000010000000"),
    ("PerkPendingRewardsDone", "0x0000000080000000"),
    ("PurchasesDone", "0x0000000000000080"),
    ("QuestCriteriaDone", "0x0000000000080000"),
    ("QuestLogDone", "0x0000000000000020"),
    ("RafActivitiesDone", "0x0000000002000000"),
    ("RafBalanceDone", "0x0000000000400000"),
    ("RafRewardsDone", "0x0000000000800000"),
    ("RevokedRafRewardsDone", "0x0000000004000000"),
    ("SettingsDone", "0x0000000000000400"),
    ("TransmogOutfitsLoadedDone", "0x0000400000000000"),
    ("TrialBoostHistoryDone", "0x0000000000040000"),
    ("VasTransactionsDone", "0x0000000000010000"),
    ("WarbandGroupsDone", "0x0000002000000000"),
    ("WarbandScenesLoadedDone", "0x0000100000000000"),
    ("WowlabsDataDone", "0x0000000020000000"),
];

fn assert_enum_family(env: &WowLuaEnv, family: &str, expected: &[(&str, &str)]) {
    let expected_lua = expected
        .iter()
        .map(|(name, value)| format!("[{name:?}] = {value:?}"))
        .collect::<Vec<_>>()
        .join(",\n                ");
    let script = format!(
        r#"
            local namespace = Enum[{family:?}]
            if type(namespace) ~= "table" then
                return "namespace:" .. type(namespace)
            end
            local expected = {{
                {expected_lua}
            }}
            for name, value in pairs(expected) do
                local actual = namespace[name]
                if type(actual) ~= "string" then
                    return name .. ":type=" .. type(actual)
                end
                if actual ~= value then
                    return name .. ":value=" .. tostring(actual)
                end
            end
            return "ok"
        "#,
        family = family,
        expected_lua = expected_lua,
    );
    let result: String = env.eval(&script).unwrap();
    assert_eq!(
        result, "ok",
        "{family} did not match the 12.0.0 source register"
    );
}

#[test]
fn test_patch_12_0_0_account_state_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    assert_enum_family(&env, "AccountStateLoadedFlags", ACCOUNT_STATE_LOADED_FLAGS);
    assert_enum_family(&env, "CreateAllAccountData", CREATE_ALL_ACCOUNT_DATA);

    let metadata_result: String = env
        .eval(
            r#"
                local expected = {
                    AccountStateLoadedFlagsMeta = 48,
                    CreateAllAccountDataMeta = 48,
                }
                for namespace_name, value in pairs(expected) do
                    local namespace = Enum[namespace_name]
                    if type(namespace) ~= "table" then
                        return namespace_name .. ":namespace=" .. type(namespace)
                    end
                    if type(namespace.NumValues) ~= "number" then
                        return namespace_name .. ".NumValues:type=" .. type(namespace.NumValues)
                    end
                    if namespace.NumValues ~= value then
                        return namespace_name .. ".NumValues:value=" .. tostring(namespace.NumValues)
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        metadata_result, "ok",
        "account-state metadata did not match the 12.0.0 source register"
    );
}
