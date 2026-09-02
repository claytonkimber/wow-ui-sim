use crate::loader::{
    LoadDiagnosticAttribution, LoadResult, MissingRequirement, MissingRequirementKind,
    NilSymbolObservation, NilSymbolObservationKind,
};
use crate::lua_api::LoaderEnv;
use crate::lua_api::methods::registry_get;
use crate::lua_api::state::{NilSymbolAccess, NilSymbolEnvironment, SimState};
use rilua::Val;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

pub(super) struct LoadingNilSymbolEnvironmentGuard {
    state: Rc<RefCell<SimState>>,
    previous: Option<NilSymbolEnvironment>,
}

impl Drop for LoadingNilSymbolEnvironmentGuard {
    fn drop(&mut self) {
        self.state.borrow_mut().loading_nil_symbol_environment = self.previous;
    }
}

pub(super) fn enter_nil_symbol_environment(
    env: &LoaderEnv<'_>,
    use_secure_env: bool,
) -> LoadingNilSymbolEnvironmentGuard {
    let state = Rc::clone(env.state());
    let environment = if use_secure_env {
        NilSymbolEnvironment::Secure
    } else {
        NilSymbolEnvironment::Public
    };
    let previous = state
        .borrow_mut()
        .loading_nil_symbol_environment
        .replace(environment);
    LoadingNilSymbolEnvironmentGuard { state, previous }
}

pub(super) fn append_nil_symbol_diagnostics(
    env: &LoaderEnv<'_>,
    addon_index: u16,
    addon_name: &str,
    start_index: usize,
    result: &mut LoadResult,
) {
    let (accesses, public_names, secure_names) = {
        let state = env.state().borrow();
        let accesses = state.nil_symbol_accesses[start_index..].to_vec();
        let public_names = publications_for_addon(&state.global_publications, addon_index);
        let secure_names = publications_for_addon(&state.secure_global_publications, addon_index);
        (accesses, public_names, secure_names)
    };
    let resolved_names = ResolvedGlobals {
        public: resolve_non_nil_names(env, public_names, raw_public_global_is_non_nil),
        secure: resolve_non_nil_names(env, secure_names, raw_secure_global_is_non_nil),
    };
    let diagnostics =
        summarize_nil_symbol_accesses(&accesses, addon_index, addon_name, &resolved_names);
    result.extend_diagnostics(diagnostics);
}

fn publications_for_addon(publications: &HashSet<(u16, String)>, addon_index: u16) -> Vec<String> {
    publications
        .iter()
        .filter(|(owner_index, _)| *owner_index == addon_index)
        .map(|(_, name)| name.clone())
        .collect()
}

fn resolve_non_nil_names(
    env: &LoaderEnv<'_>,
    names: Vec<String>,
    is_non_nil: fn(&LoaderEnv<'_>, &str) -> bool,
) -> HashSet<String> {
    names
        .into_iter()
        .filter(|name| is_non_nil(env, name))
        .collect()
}

struct ResolvedGlobals {
    public: HashSet<String>,
    secure: HashSet<String>,
}

fn summarize_nil_symbol_accesses(
    accesses: &[NilSymbolAccess],
    addon_index: u16,
    addon_name: &str,
    resolved_names: &ResolvedGlobals,
) -> crate::loader::LoadDiagnostics {
    let mut observations = BTreeMap::new();
    let mut requirements = BTreeMap::new();

    for access in accesses {
        if access.addon_index != Some(addon_index) || is_resolved_global(access, resolved_names) {
            continue;
        }

        let attribution = diagnostic_attribution(addon_name, access);
        match classify_nil_symbol_access(access) {
            ClassifiedNilSymbol::Observation(kind) => {
                observations
                    .entry((access.environment, kind))
                    .or_insert(attribution);
            }
            ClassifiedNilSymbol::Requirement(kind) => {
                requirements
                    .entry((access.environment, kind))
                    .or_insert(attribution);
            }
        }
    }

    crate::loader::LoadDiagnostics {
        warnings: Vec::new(),
        nil_symbol_observations: observations
            .into_iter()
            .map(|((_, kind), attribution)| NilSymbolObservation { kind, attribution })
            .collect(),
        missing_requirements: requirements
            .into_iter()
            .map(|((_, kind), attribution)| MissingRequirement { kind, attribution })
            .collect(),
    }
}

fn is_resolved_global(access: &NilSymbolAccess, resolved_names: &ResolvedGlobals) -> bool {
    if !matches!(access.container.as_str(), "_G" | "__secureenv") || access.key.starts_with("C_") {
        return false;
    }

    match access.environment {
        NilSymbolEnvironment::Public => resolved_names.public.contains(&access.key),
        NilSymbolEnvironment::Secure => resolved_names.secure.contains(&access.key),
    }
}

fn raw_public_global_is_non_nil(env: &LoaderEnv<'_>, name: &str) -> bool {
    env.with_state(|state| {
        let key = state.gc.intern_string(name.as_bytes());
        let value = state
            .gc
            .tables
            .get(state.global)
            .map(|globals| globals.get_str(key, &state.gc.string_arena))
            .unwrap_or(Val::Nil);
        Ok::<bool, std::convert::Infallible>(!matches!(value, Val::Nil))
    })
    .unwrap_or(false)
}

fn raw_secure_global_is_non_nil(env: &LoaderEnv<'_>, name: &str) -> bool {
    env.with_state(|state| {
        let Val::Table(secureenv_ref) = registry_get(state, "__secureenv") else {
            return Ok::<bool, std::convert::Infallible>(false);
        };
        let key = state.gc.intern_string(name.as_bytes());
        let value = state
            .gc
            .tables
            .get(secureenv_ref)
            .map(|secureenv| secureenv.get_str(key, &state.gc.string_arena))
            .unwrap_or(Val::Nil);
        Ok::<bool, std::convert::Infallible>(!matches!(value, Val::Nil))
    })
    .unwrap_or(false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ClassifiedNilSymbol {
    Observation(NilSymbolObservationKind),
    Requirement(MissingRequirementKind),
}

fn classify_nil_symbol_access(access: &NilSymbolAccess) -> ClassifiedNilSymbol {
    if matches!(access.container.as_str(), "_G" | "__secureenv") {
        if access.key.starts_with("C_") {
            return ClassifiedNilSymbol::Requirement(MissingRequirementKind::CNamespace {
                namespace: access.key.clone(),
            });
        }
        return ClassifiedNilSymbol::Observation(NilSymbolObservationKind::Global {
            name: access.key.clone(),
        });
    }

    if access.container.starts_with("C_") {
        return ClassifiedNilSymbol::Requirement(MissingRequirementKind::CMethod {
            namespace: access.container.clone(),
            method: access.key.clone(),
        });
    }

    ClassifiedNilSymbol::Observation(NilSymbolObservationKind::Global {
        name: format!("{}.{}", access.container, access.key),
    })
}

fn diagnostic_attribution(addon_name: &str, access: &NilSymbolAccess) -> LoadDiagnosticAttribution {
    LoadDiagnosticAttribution {
        addon_name: addon_name.to_string(),
        environment: access.environment,
        source: access.source.as_deref().map(summarize_chunk_source),
        line: access.line,
    }
}

fn summarize_chunk_source(source: &str) -> String {
    let stripped = source.trim_start_matches(['@', '=']);
    let path = PathBuf::from(stripped);
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| stripped.to_string())
}
