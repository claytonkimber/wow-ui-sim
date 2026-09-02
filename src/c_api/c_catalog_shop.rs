//! `C_CatalogShop` virtual-currency product surface.

use crate::c_api::helpers::ensure_namespace;
use crate::lua_api::methods::create_table;
use crate::lua_bridge::table_set_rust_fn_static;
use rilua::LuaResult;
use rilua::vm::state::LuaState;

pub(crate) fn register_c_catalog_shop_surface(state: &mut LuaState) -> LuaResult<()> {
    let table_ref = ensure_namespace(state, "C_CatalogShop")?;
    table_set_rust_fn_static(
        state,
        table_ref,
        "GetVCProductInfos",
        c_catalog_shop_get_vc_product_infos,
    )
}

fn c_catalog_shop_get_vc_product_infos(state: &mut LuaState) -> LuaResult<u32> {
    let product_infos = create_table(state);
    state.push(product_infos);
    Ok(1)
}
