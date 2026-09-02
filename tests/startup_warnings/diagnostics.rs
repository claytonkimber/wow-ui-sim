use super::drain_test_errors;
use std::path::Path;
use wow_ui_sim::loader::{
    LoadDiagnostics, LoadResult, MissingRequirement, NilSymbolObservation, load_addon,
};
use wow_ui_sim::lua_api::WowLuaEnv;

#[derive(Debug, Default)]
pub(super) struct StartupDiagnostics {
    pub(super) warnings: Vec<String>,
    pub(super) nil_symbol_observations: Vec<NilSymbolObservation>,
    pub(super) missing_requirements: Vec<MissingRequirement>,
}

impl StartupDiagnostics {
    pub(super) fn extend(&mut self, diagnostics: StartupDiagnostics) {
        self.warnings.extend(diagnostics.warnings);
        self.nil_symbol_observations
            .extend(diagnostics.nil_symbol_observations);
        self.missing_requirements
            .extend(diagnostics.missing_requirements);
    }

    fn extend_load_result(&mut self, phase: &str, result: LoadResult) {
        self.warnings.extend(
            result
                .warnings
                .into_iter()
                .map(|warning| format!("[{phase}] {warning}")),
        );
        self.nil_symbol_observations
            .extend(result.nil_symbol_observations);
        self.missing_requirements
            .extend(result.missing_requirements);
    }

    fn extend_runtime(&mut self, phase: &str, diagnostics: LoadDiagnostics) {
        self.warnings.extend(
            diagnostics
                .warnings
                .into_iter()
                .map(|warning| format!("[{phase}] {warning}")),
        );
        self.nil_symbol_observations
            .extend(diagnostics.nil_symbol_observations);
        self.missing_requirements
            .extend(diagnostics.missing_requirements);
    }
}

pub(super) fn collect_handler_and_runtime_diagnostics(
    env: &WowLuaEnv,
    phase: &str,
) -> StartupDiagnostics {
    let mut diagnostics = StartupDiagnostics {
        warnings: drain_test_errors(env)
            .into_iter()
            .map(|warning| format!("[{phase}] {warning}"))
            .collect(),
        ..StartupDiagnostics::default()
    };
    diagnostics.extend_runtime(phase, env.drain_runtime_addon_diagnostics());
    diagnostics
}

pub(super) fn collect_addon_load_diagnostics(
    env: &WowLuaEnv,
    name: &str,
    toc_path: &Path,
) -> StartupDiagnostics {
    let result = load_addon(&env.loader_env(), toc_path);
    let load_handler_diagnostics =
        collect_handler_and_runtime_diagnostics(env, &format!("load {name} handler"));

    let result = match result {
        Ok(result) => result,
        Err(error) => {
            let mut diagnostics = StartupDiagnostics {
                warnings: vec![format!("[load {name}] FAILED: {error}")],
                ..StartupDiagnostics::default()
            };
            diagnostics.extend(load_handler_diagnostics);
            return diagnostics;
        }
    };

    let mut diagnostics = StartupDiagnostics::default();
    diagnostics.extend_load_result(&format!("load {name}"), result);
    diagnostics.extend(load_handler_diagnostics);
    if let Err(error) = env.fire_event_with_args("ADDON_LOADED", &[env.lua_string(name)]) {
        diagnostics
            .warnings
            .push(format!("[ADDON_LOADED {name}] FAILED: {error}"));
    }
    diagnostics.extend(collect_handler_and_runtime_diagnostics(
        env,
        &format!("ADDON_LOADED {name}"),
    ));
    diagnostics
}
