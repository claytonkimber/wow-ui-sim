//! Temporary Catalog Shop product-card layout guard.
//!
//! Catalog shop cards can arrive without a product ID in the simulated startup
//! data path. Keep the nil-safe layout guard isolated until that data is seeded.

use crate::lua_api::{LoaderEnv, WowLuaEnv};

const CATALOG_SHOP_DEFAULTS_LUA: &str = r#"
SOUNDKIT = SOUNDKIT or {}
if SOUNDKIT.CATALOG_SHOP_SELECT_NAV_MENU == nil then
    SOUNDKIT.CATALOG_SHOP_SELECT_NAV_MENU = 303824
end
if SOUNDKIT.CATALOG_SHOP_SELECT_GENERIC_UI_BUTTON == nil then
    SOUNDKIT.CATALOG_SHOP_SELECT_GENERIC_UI_BUTTON = 303826
end
if SOUNDKIT.CATALOG_SHOP_OPEN_LOADING_SCREEN == nil then
    SOUNDKIT.CATALOG_SHOP_OPEN_LOADING_SCREEN = 303820
end
if SOUNDKIT.CATALOG_SHOP_LOADING_SCREEN_LOOP == nil then
    SOUNDKIT.CATALOG_SHOP_LOADING_SCREEN_LOOP = 303821
end
if SOUNDKIT.CATALOG_SHOP_OPEN_SHOP_AFTER_LOAD == nil then
    SOUNDKIT.CATALOG_SHOP_OPEN_SHOP_AFTER_LOAD = 303822
end
if SOUNDKIT.CATALOG_SHOP_GOLD_SHIMMER_START == nil then
    SOUNDKIT.CATALOG_SHOP_GOLD_SHIMMER_START = 306261
end
if SOUNDKIT.CATALOG_SHOP_GOLD_SHIMMER_LOOP == nil then
    SOUNDKIT.CATALOG_SHOP_GOLD_SHIMMER_LOOP = 303827
end
if SOUNDKIT.CATALOG_SHOP_GOLD_SHIMMER_END == nil then
    SOUNDKIT.CATALOG_SHOP_GOLD_SHIMMER_END = 306262
end

local catalogShopConstantsDefaults = {
    ScrollViewType = {
        List = 1,
        Grid = 2,
    },
    ScrollViewElementType = {
        Header = 1,
        Product = 2,
    },
    Default = {
        PreviewBackgroundTexture = "shop-bg-map-blue",
    },
    CardTemplate = {
        Header = "CatalogShopSectionHeaderTemplate",
        HeaderPersonalized = "CatalogShopSectionHeaderOptOutLinkTemplate",
        Wide = "WideCatalogShopProductCardTemplate",
        WideCardToken = "WideWoWTokenCatalogShopCardTemplate",
        WideCardSubscription = "WideSubscriptionCatalogShopCardTemplate",
        WideCardGameTime = "WideGameTimeCatalogShopCardTemplate",
        Small = "SmallCatalogShopProductCardTemplate",
        SmallServices = "SmallCatalogShopServicesCardTemplate",
        SmallSubscription = "SmallCatalogShopSubscriptionCardTemplate",
        SmallGameTime = "SmallCatalogShopGameTimeCardTemplate",
        SmallTender = "SmallCatalogShopTenderCardTemplate",
        SmallToys = "SmallCatalogShopToysCardTemplate",
        SmallAccess = "SmallCatalogShopAccessCardTemplate",
        SmallDecor = "SmallCatalogShopDecorCardTemplate",
        SmallRoom = "SmallCatalogShopRoomCardTemplate",
        SmallExteriorType = "SmallCatalogShopExteriorTypeCardTemplate",
        Details = "DetailsCatalogShopProductCardTemplate",
        DetailsServices = "DetailsCatalogShopServicesCardTemplate",
        DetailsSubscription = "DetailsCatalogShopSubscriptionCardTemplate",
        DetailsGameTime = "DetailsCatalogShopGameTimeCardTemplate",
        DetailsTender = "DetailsCatalogShopTenderCardTemplate",
        DetailsToys = "DetailsCatalogShopToysCardTemplate",
        DetailsAccess = "DetailsCatalogShopAccessCardTemplate",
        DetailsDecor = "DetailsCatalogShopDecorCardTemplate",
        DetailsRoom = "DetailsCatalogShopRoomCardTemplate",
        DetailsExteriorType = "DetailsCatalogShopExteriorTypeCardTemplate",
    },
    ModelSceneContext = {
        SmallCard = 1,
        WideCard = 2,
        PreviewScene = 3,
    },
    ScreenPadding = {
        Horizontal = 100,
        Vertical = 100,
    },
    DefaultActorTag = {
        Pet = "pet",
        Mount = "mount",
        Toy = "actor",
        Celebrate = "fanfare",
        Transmog = "transmog",
        Decor = "decor",
    },
    ProductType = {
        Pet = "Pet",
        Mount = "Mount",
        Toy = "Toy",
        Transmog = "Transmog",
        Services = "Services",
        Token = "WoW Token",
        Bundle = "Bundle",
        Subscription = "Subscription",
        TradersTenders = "Tender",
        Decor = "Decor",
        Access = "Access",
        GameTime = "Game Time",
        Room = "Room",
        HousingExteriorType = "Housing Exteriors",
    },
    GameTypes = {
        Classic = "classic",
        Modern = "modern",
    },
    GameTypeGlobalStringTag = {
        Classic = CATALOG_SHOP_WOW_FLAVOR_CLASSIC,
        Modern = CATALOG_SHOP_WOW_FLAVOR_MODERN,
    },
    ShopGlobalStringTag = {
        MissingLicenseCaptionText = CATALOG_SHOP_MISSING_LICENSE_ITEM,
    },
    DefaultCameraTag = {
        Primary = "primary",
    },
    DefaultAnimID = {
        MountSelfIdle = 618,
        MountSpecialAnimKit = 1371,
        PetDefault = 23877,
    },
    Celebrate = {
        CreatureID = 27823,
        SpellVisualID = 259750,
    },
    NoResults = {
        BackgroundTexture = "shop-bg-map-blue",
    },
    LicenseTermTypes = {
        NotApplicable = 0,
        Days = 1,
        Years = 2,
        Fixed = 3,
        Hours = 4,
        Minutes = 5,
        Weeks = 6,
        Months = 7,
    },
    CategoryLinks = {
        Pets = "pets",
        Mounts = "mounts",
        Transmogs = "transmogs",
        Subscriptions = "subscriptions",
        GameTime = "gametimeoffers",
        GameUpgrades = "game upgrades",
        Services = "services",
        Featured = "featured",
        Housing = "housing",
    },
}

if type(CatalogShopConstants) ~= "table" then
    CatalogShopConstants = {}
end

for key, value in pairs(catalogShopConstantsDefaults) do
    if CatalogShopConstants[key] == nil then
        CatalogShopConstants[key] = value
    end
end
"#;

const CATALOG_SHOP_PRODUCT_CARD_DEFAULTS_WORKAROUND_LUA: &str = r#"
if rawget(_G, "__wow_catalog_shop_product_card_defaults_wrapped") then
    return
end

if type(CatalogShopDefaultProductCardMixin) ~= "table"
    or type(CatalogShopDefaultProductCardMixin.Layout) ~= "function" then
    return
end

local original_layout = CatalogShopDefaultProductCardMixin.Layout

local function resolve_product_id(card)
    if type(card.productInfo) == "table"
        and type(card.productInfo.catalogShopProductID) == "number" then
        return card.productInfo.catalogShopProductID
    end

    if type(card.GetElementData) == "function" then
        local elementData = card:GetElementData()
        if type(elementData) == "table" then
            local productID = elementData.catalogShopProductID or elementData.productID
            if type(productID) == "number" then
                if type(card.productInfo) == "table" then
                    card.productInfo.catalogShopProductID = productID
                end
                return productID
            end
        end
    end

    return nil
end

CatalogShopDefaultProductCardMixin.Layout = function(self, ...)
    if resolve_product_id(self) == nil then
        return
    end
    return original_layout(self, ...)
end

rawset(_G, "__wow_catalog_shop_product_card_defaults_wrapped", true)
"#;

const CATALOG_SHOP_ESCAPE_DEFAULTS_LUA: &str = r#"
if type(CatalogShopInboundInterface) == "table" then
    local originalEscapePressed = CatalogShopInboundInterface.EscapePressed
    if type(originalEscapePressed) == "function" and not rawget(_G, "__wow_catalog_shop_escape_wrapped") then
        CatalogShopInboundInterface.EscapePressed = function(...)
            if CatalogShopFrame == nil then
                return false
            end
            return originalEscapePressed(...)
        end
        rawset(_G, "__wow_catalog_shop_escape_wrapped", true)
    end
end
"#;

pub(crate) fn apply_bootstrap(lua: &mut rilua::Lua) -> crate::Result<()> {
    lua.exec(CATALOG_SHOP_DEFAULTS_LUA)?;
    Ok(())
}

pub(crate) fn patch(env: &WowLuaEnv) {
    let _ = env.exec(CATALOG_SHOP_ESCAPE_DEFAULTS_LUA);
    let _ = env.exec(CATALOG_SHOP_PRODUCT_CARD_DEFAULTS_WORKAROUND_LUA);
}

pub(crate) fn patch_for_runtime_addon_load(env: &LoaderEnv<'_>) {
    let _ = env.exec(CATALOG_SHOP_ESCAPE_DEFAULTS_LUA);
    let _ = env.exec(CATALOG_SHOP_PRODUCT_CARD_DEFAULTS_WORKAROUND_LUA);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_catalog_shop_defaults() {
        let env = WowLuaEnv::new().expect("lua env should initialize");
        {
            let mut lua = env.lua.borrow_mut();
            super::apply_bootstrap(&mut lua).expect("catalog shop defaults should apply");
        }

        let defaults: (i64, i64, i64, i64, i64, i64, i64, i64, i64, String) = env
            .eval(
                r#"
                return SOUNDKIT.CATALOG_SHOP_SELECT_NAV_MENU,
                    SOUNDKIT.CATALOG_SHOP_SELECT_GENERIC_UI_BUTTON,
                    SOUNDKIT.CATALOG_SHOP_OPEN_LOADING_SCREEN,
                    SOUNDKIT.CATALOG_SHOP_LOADING_SCREEN_LOOP,
                    SOUNDKIT.CATALOG_SHOP_OPEN_SHOP_AFTER_LOAD,
                    SOUNDKIT.CATALOG_SHOP_GOLD_SHIMMER_START,
                    SOUNDKIT.CATALOG_SHOP_GOLD_SHIMMER_LOOP,
                    SOUNDKIT.CATALOG_SHOP_GOLD_SHIMMER_END,
                    CatalogShopConstants.ScrollViewElementType.Product,
                    CatalogShopConstants.CardTemplate.Details
                "#,
            )
            .expect("catalog shop defaults should be readable");

        assert_eq!(
            defaults,
            (
                303824,
                303826,
                303820,
                303821,
                303822,
                306261,
                303827,
                306262,
                2,
                "DetailsCatalogShopProductCardTemplate".to_string(),
            )
        );
    }

    fn install_layout_fixture(env: &WowLuaEnv) {
        env.exec(
            r#"
            layout_calls = 0
            layout_arg = nil
            CatalogShopDefaultProductCardMixin = {
                Layout = function(self, value)
                    layout_calls = layout_calls + 1
                    layout_arg = value
                    return "laid out"
                end,
            }
            "#,
        )
        .expect("catalog product card fixture should install");
    }

    #[test]
    fn skips_layout_without_product_id() {
        let env = WowLuaEnv::new().expect("lua env should initialize");
        install_layout_fixture(&env);
        patch(&env);

        let (result, layout_calls): (Option<String>, i64) = env
            .eval(
                r#"
                local card = {}
                return CatalogShopDefaultProductCardMixin.Layout(card, "ignored"), layout_calls
                "#,
            )
            .expect("catalog card layout should run");

        assert_eq!(result, None);
        assert_eq!(layout_calls, 0);
    }

    #[test]
    fn uses_product_info_id_to_call_original_layout() {
        let env = WowLuaEnv::new().expect("lua env should initialize");
        install_layout_fixture(&env);
        patch(&env);

        let (result, layout_calls, layout_arg): (String, i64, String) = env
            .eval(
                r#"
                local card = { productInfo = { catalogShopProductID = 123 } }
                return CatalogShopDefaultProductCardMixin.Layout(card, "from-product-info"),
                    layout_calls,
                    layout_arg
                "#,
            )
            .expect("catalog card layout should use productInfo ID");

        assert_eq!(result, "laid out");
        assert_eq!(layout_calls, 1);
        assert_eq!(layout_arg, "from-product-info");
    }

    #[test]
    fn copies_element_data_product_id_before_layout() {
        let env = WowLuaEnv::new().expect("lua env should initialize");
        install_layout_fixture(&env);
        patch(&env);

        let (layout_calls, product_id): (i64, i64) = env
            .eval(
                r#"
                local card = {
                    productInfo = {},
                    GetElementData = function()
                        return { productID = 456 }
                    end,
                }
                CatalogShopDefaultProductCardMixin.Layout(card, "from-element-data")
                return layout_calls, card.productInfo.catalogShopProductID
                "#,
            )
            .expect("catalog card layout should use element data ID");

        assert_eq!(layout_calls, 1);
        assert_eq!(product_id, 456);
    }
}
