#!/usr/bin/env python3
"""Generate the WoW 12.0.0 API occurrence source register from wowless history."""

from __future__ import annotations

import argparse
import copy
import json
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:
    print("Error: PyYAML is required.", file=sys.stderr)
    raise SystemExit(1)


WOWLESS_DIR = Path(os.path.expanduser("~/Repos/wowless"))
OUTPUT_PATH = Path("data/patch-api/sources/12.0.0-register.json")
PRODUCT_PATH = "data/products/wow"
SOURCE_FILES = (
    "apis",
    "cvars",
    "docs",
    "events",
    "globals",
    "luaobjects",
    "structures",
    "uiobjects",
)
BASE_COMMIT = "d9efaadf92f558e2b4fbef622c7b8af0e843849a"
PATCH_COMMITS = (
    "78e503fb24e467ec0354148f7ba41b77a3158ff6",
    "16ae143430a9e3704f639c5452a5315408d5dc18",
    "f39e3453ebb67f1be70e127e146092a8129954bb",
    "33cf699b1d91d4743acb5c003339b1f5ed2c28c2",
    "03bb5214f7a951ca5b5a6d38dc7ca56af164b281",
    "a6d2717d06f9255e507ab07f811c1bafaea64939",
)
EXPECTED_SNAPSHOTS = {
    BASE_COMMIT: ("11.2.7", "65299", 110207),
    PATCH_COMMITS[0]: ("12.0.0", "65512", 120000),
    PATCH_COMMITS[1]: ("12.0.0", "65535", 120000),
    PATCH_COMMITS[2]: ("12.0.0", "65560", 120000),
    PATCH_COMMITS[3]: ("12.0.0", "65655", 120000),
    PATCH_COMMITS[4]: ("12.0.0", "65699", 120000),
    PATCH_COMMITS[5]: ("12.0.0", "65727", 120000),
}
LUA_PATH = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$")


class _StringBoolLoader(yaml.SafeLoader):
    """Keep YAML 1.1 On/Off/Yes/No values as strings."""


_StringBoolLoader.yaml_implicit_resolvers = {
    key: [(tag, regexp) for tag, regexp in resolvers if tag != "tag:yaml.org,2002:bool"]
    for key, resolvers in yaml.SafeLoader.yaml_implicit_resolvers.copy().items()
}
_StringBoolLoader.add_implicit_resolver(
    "tag:yaml.org,2002:bool",
    re.compile(r"^(?:true|false)$"),
    list("tf"),
)


def _canonical(value: Any) -> Any:
    if isinstance(value, dict):
        return {str(key): _canonical(value[key]) for key in sorted(value, key=str)}
    if isinstance(value, list):
        return [_canonical(item) for item in value]
    return value


def _record(path: str, category: str, value: Any, metadata: dict | None = None) -> dict:
    if not LUA_PATH.fullmatch(path):
        raise ValueError(f"invalid Lua-style occurrence path: {path}")
    return {
        "path": path,
        "category": category,
        "value": _canonical(value),
        "metadata": _canonical(metadata or {}),
    }


def _insert(records: dict[str, dict], record: dict) -> None:
    path = record["path"]
    if path in records:
        raise ValueError(f"duplicate occurrence path: {path}")
    records[path] = record


def _without(value: dict, *keys: str) -> dict:
    return {key: item for key, item in value.items() if key not in keys}


def _extract_canonical_lists(snapshot: dict, records: dict[str, dict]) -> None:
    for api in snapshot.get("apis", []):
        namespace = api.get("namespace")
        name = api["name"]
        path = f"{namespace}.{name}" if namespace else name
        _insert(records, _record(path, "api", _without(api, "namespace", "name")))
    for item in snapshot.get("globals", []):
        _insert(records, _record(item["name"], "global", _without(item, "name")))
    for item in snapshot.get("constants", []):
        group = item.get("group")
        path = f"Constants.{group}.{item['name']}" if group else item["name"]
        _insert(records, _record(path, "constant", _without(item, "group", "name")))
    for item in snapshot.get("enums", []):
        _insert(
            records,
            _record(
                f"Enum.{item['namespace']}.{item['name']}",
                "enum",
                _without(item, "namespace", "name"),
            ),
        )
    for item in snapshot.get("cvars", []):
        _insert(records, _record(item["name"], "cvar", _without(item, "name")))
    for item in snapshot.get("events", []):
        _insert(records, _record(item["name"], "event", _without(item, "name")))
    _extract_canonical_objects(snapshot.get("structures", []), records, "structure")
    _extract_canonical_objects(snapshot.get("uiobjects", []), records, "uiobject")
    _extract_canonical_objects(snapshot.get("luaobjects", []), records, "luaobject")
    for item in snapshot.get("typedefs", []):
        _insert(
            records,
            _record(f"typedef.{item['name']}", "typedef", _without(item, "name")),
        )
    for item in snapshot.get("script_objects", []):
        _insert(
            records,
            _record(
                f"script_object.{item['name']}",
                "script-object",
                _without(item, "name"),
            ),
        )


def _extract_canonical_objects(items: list[dict], records: dict[str, dict], kind: str) -> None:
    member_key = "fields" if kind == "structure" else "methods"
    for item in items:
        name = item["name"]
        _insert(records, _record(name, kind, _without(item, "name", member_key)))
        for member in item.get(member_key, []):
            category = "structure-field" if kind == "structure" else f"{kind}-method"
            _insert(
                records,
                _record(f"{name}.{member['name']}", category, _without(member, "name")),
            )


def _extract_raw_wowless(snapshot: dict, records: dict[str, dict]) -> None:
    for path, value in (snapshot.get("apis") or {}).items():
        _insert(records, _record(path, "api", value))
    _extract_raw_globals(snapshot.get("globals") or {}, records)
    for path, value in (snapshot.get("cvars") or {}).items():
        _insert(records, _record(path, "cvar", value))
    for path, value in (snapshot.get("events") or {}).items():
        _insert(records, _record(path, "event", value))
    _extract_raw_objects(snapshot.get("structures") or {}, records, "structure")
    _extract_raw_objects(snapshot.get("uiobjects") or {}, records, "uiobject")
    _extract_raw_objects(snapshot.get("luaobjects") or {}, records, "luaobject")
    _extract_raw_docs(snapshot.get("docs") or {}, records)


def _extract_raw_globals(globals_data: dict, records: dict[str, dict]) -> None:
    constants = globals_data.get("Constants", {})
    for group, values in constants.items():
        for name, value in values.items():
            _insert(records, _record(f"Constants.{group}.{name}", "constant", value))
    enums = globals_data.get("Enum", {})
    for namespace, values in enums.items():
        for name, value in values.items():
            _insert(records, _record(f"Enum.{namespace}.{name}", "enum", value))
    for name, value in globals_data.items():
        if name not in {"Constants", "Enum"}:
            _insert(records, _record(name, "global", value))


def _extract_raw_objects(objects: dict, records: dict[str, dict], kind: str) -> None:
    member_key = "fields" if kind == "structure" else "methods"
    for name, value in objects.items():
        value = value or {}
        _insert(records, _record(name, kind, _without(value, member_key)))
        members = value.get(member_key) or ([] if kind == "structure" else {})
        if kind == "structure":
            if isinstance(members, dict):
                structure_fields = members.items()
            else:
                structure_fields = (
                    (member["name"], _without(member, "name")) for member in members
                )
            for member_name, member_value in structure_fields:
                _insert(
                    records,
                    _record(
                        f"{name}.{member_name}",
                        "structure-field",
                        member_value,
                    ),
                )
        else:
            for member_name, member_value in members.items():
                _insert(
                    records,
                    _record(f"{name}.{member_name}", f"{kind}-method", member_value),
                )


def _extract_raw_docs(docs: dict, records: dict[str, dict]) -> None:
    for name, value in docs.get("typedefs", {}).items():
        _insert(records, _record(f"typedef.{name}", "typedef", value))
    for name, value in docs.get("script_objects", {}).items():
        _insert(records, _record(f"script_object.{name}", "script-object", value))
    _add_docs_extra_records(docs.get("lies", {}), records)
    _merge_raw_docs_metadata(docs, records)


def _add_docs_extra_records(lies: dict, records: dict[str, dict]) -> None:
    categories = {
        "extra_apis": "docs-extra-api",
        "extra_enums": "docs-extra-enum",
        "extra_events": "docs-extra-event",
        "extra_script_objects": "docs-extra-script-object",
    }
    for section, category in categories.items():
        for name, value in (lies.get(section) or {}).items():
            path = f"docs.{section}.{name}"
            _insert(records, _record(path, category, value))


def _merge_raw_docs_metadata(docs: dict, records: dict[str, dict]) -> None:
    lies = docs.get("lies", {})
    docs_lies: dict[str, Any] = {}
    docs_lies.update(lies.get("apis") or {})
    docs_lies.update(lies.get("events") or {})
    for owner, methods in (lies.get("uiobjects") or {}).items():
        for method, value in (methods or {}).items():
            docs_lies[f"{owner}.{method}"] = value
    merged = merge_docs_lie_metadata(list(records.values()), docs_lies)
    records.clear()
    records.update({record["path"]: record for record in merged})
    _merge_method_metadata(
        records,
        docs.get("ignore_script_object_methods") or {},
        "ignored_script_object_method",
        True,
    )
    _merge_method_metadata(
        records,
        docs.get("uiobject_method_reassignments") or {},
        "uiobject_method_reassignment",
        None,
    )


def _merge_method_metadata(
    records: dict[str, dict],
    mapping: dict,
    metadata_key: str,
    fixed_value: Any,
) -> None:
    for owner, methods in mapping.items():
        entries = methods.items() if isinstance(methods, dict) else ((method, fixed_value) for method in methods)
        for method, value in entries:
            path = f"{owner}.{method}"
            if path not in records:
                records[path] = _record(path, "docs-method-metadata", None)
            records[path]["metadata"][metadata_key] = fixed_value if fixed_value is not None else value
            records[path]["metadata"] = _canonical(records[path]["metadata"])


def extract_symbols(snapshot: dict) -> list[dict]:
    """Extract deterministic occurrence records from canonical or raw wowless data."""
    records: dict[str, dict] = {}
    if isinstance(snapshot.get("apis"), list):
        _extract_canonical_lists(snapshot, records)
    else:
        _extract_raw_wowless(snapshot, records)
    return [records[path] for path in sorted(records)]


def merge_docs_lie_metadata(records: list[dict], docs_lies: dict[str, Any]) -> list[dict]:
    """Merge docs-lie annotations into the matching occurrence without duplication."""
    merged = {record["path"]: copy.deepcopy(record) for record in records}
    for path, value in docs_lies.items():
        if path not in merged:
            merged[path] = _record(path, "docs-lie", None)
        merged[path]["metadata"]["docs_lie"] = _canonical(value)
        merged[path]["metadata"] = _canonical(merged[path]["metadata"])
    return [merged[path] for path in sorted(merged)]


def _signature(record: dict) -> str:
    return json.dumps(
        {
            "category": record["category"],
            "value": record["value"],
            "metadata": record["metadata"],
        },
        sort_keys=True,
        separators=(",", ":"),
    )


def _payload(record: dict) -> dict:
    return {
        "category": record["category"],
        "value": record["value"],
        "metadata": record["metadata"],
    }


def _occurrence(
    direction: str,
    record: dict,
    detail: str,
    *,
    before: dict | None = None,
    after: dict | None = None,
) -> dict:
    occurrence = {
        "direction": direction,
        "category": record["category"],
        "symbol": record["path"],
        "detail": detail,
    }
    if before is not None:
        occurrence["before"] = _payload(before)
    if after is not None:
        occurrence["after"] = _payload(after)
    return occurrence


def _diff_records(
    before_records: list[dict],
    after_records: list[dict],
    intermediate_records: list[list[dict]],
) -> list[dict]:
    before = {record["path"]: record for record in before_records}
    after = {record["path"]: record for record in after_records}
    occurrences = []
    for path in sorted(after.keys() - before.keys()):
        record = after[path]
        occurrences.append(
            _occurrence(
                "added",
                record,
                f"{record['category']} added in 12.0.0.",
                after=record,
            )
        )
    for path in sorted(before.keys() & after.keys()):
        if _signature(before[path]) != _signature(after[path]):
            record = after[path]
            occurrences.append(
                _occurrence(
                    "changed",
                    record,
                    f"{record['category']} changed in 12.0.0.",
                    before=before[path],
                    after=record,
                )
            )
    for path in sorted(before.keys() - after.keys()):
        record = before[path]
        occurrences.append(
            _occurrence(
                "removed",
                record,
                f"{record['category']} removed in 12.0.0.",
                before=record,
            )
        )
    occurrences.extend(_transient_occurrences(before, after, intermediate_records))
    return sorted(occurrences, key=lambda item: (item["direction"], item["symbol"]))


def _transient_occurrences(
    before: dict[str, dict],
    after: dict[str, dict],
    intermediate_records: list[list[dict]],
) -> list[dict]:
    intermediate_maps = [
        {record["path"]: record for record in records} for records in intermediate_records
    ]
    intermediate_paths = set().union(*(mapping.keys() for mapping in intermediate_maps)) if intermediate_maps else set()
    transient_paths = intermediate_paths - before.keys() - after.keys()
    occurrences = []
    for path in sorted(transient_paths):
        record = next(mapping[path] for mapping in intermediate_maps if path in mapping)
        detail = (
            f"Transient {record['category']} existed in an intermediate 12.0.0 snapshot "
            "but was absent at both patch endpoints."
        )
        occurrences.append(_occurrence("added", record, detail, after=record))
        occurrences.append(_occurrence("removed", record, detail, before=record))
    return occurrences


def diff_endpoint_symbols(
    before: dict,
    after: dict,
    *,
    intermediate_snapshots: list[dict] | None = None,
) -> list[dict]:
    """Diff endpoint snapshots and preserve transient-only intermediate symbols."""
    intermediates = intermediate_snapshots or []
    return _diff_records(
        extract_symbols(before),
        extract_symbols(after),
        [extract_symbols(snapshot) for snapshot in intermediates],
    )


def build_register(
    before: dict,
    after: dict,
    *,
    intermediate_snapshots: list[dict] | None = None,
    docs_lies: dict[str, Any] | None = None,
) -> dict:
    """Build the deterministic occurrence register body."""
    intermediates = intermediate_snapshots or []
    before_records = extract_symbols(before)
    after_records = extract_symbols(after)
    intermediate_records = [extract_symbols(snapshot) for snapshot in intermediates]
    if docs_lies:
        before_records = merge_docs_lie_metadata(before_records, docs_lies)
        after_records = merge_docs_lie_metadata(after_records, docs_lies)
        intermediate_records = [
            merge_docs_lie_metadata(records, docs_lies) for records in intermediate_records
        ]
    occurrences = _diff_records(before_records, after_records, intermediate_records)
    return {
        "schema": "patch-api-source-register/v1",
        "patch": "12.0.0",
        "category_counts": dict(sorted(Counter(item["category"] for item in occurrences).items())),
        "direction_counts": dict(sorted(Counter(item["direction"] for item in occurrences).items())),
        "occurrences": occurrences,
    }


def _git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result.stdout


def _read_yaml_at_commit(repo: Path, commit: str, relative_path: str) -> Any:
    contents = _git(repo, "show", f"{commit}:{relative_path}")
    return yaml.load(contents, Loader=_StringBoolLoader)


def _load_snapshot(repo: Path, commit: str) -> tuple[dict, dict]:
    build = _read_yaml_at_commit(repo, commit, f"{PRODUCT_PATH}/build.yaml")
    expected = EXPECTED_SNAPSHOTS[commit]
    actual = (str(build["version"]), str(build["build"]), int(build["tocversion"]))
    if actual != expected:
        raise RuntimeError(f"snapshot {commit} metadata mismatch: expected {expected}, got {actual}")
    snapshot = {
        name: _read_yaml_at_commit(repo, commit, f"{PRODUCT_PATH}/{name}.yaml")
        for name in SOURCE_FILES
    }
    metadata = {
        "commit": commit,
        "version": actual[0],
        "build": actual[1],
        "tocversion": actual[2],
        "client_date": str(build.get("date", "")),
        "client_hash": str(build.get("hash", "")),
    }
    return snapshot, metadata


def generate_register(repo: Path) -> dict:
    base_snapshot, base_metadata = _load_snapshot(repo, BASE_COMMIT)
    patch_snapshots = []
    snapshot_metadata = []
    for commit in PATCH_COMMITS:
        snapshot, metadata = _load_snapshot(repo, commit)
        patch_snapshots.append(snapshot)
        snapshot_metadata.append(metadata)
    register = build_register(
        base_snapshot,
        patch_snapshots[-1],
        intermediate_snapshots=patch_snapshots[:-1],
    )
    register["source"] = {
        "repository": _git(repo, "remote", "get-url", "origin").strip(),
        "product": "wow",
        "boundary": "last explicit retail 11.2.7 snapshot to final explicit retail 12.0.0 snapshot before 12.0.1",
        "base": base_metadata,
        "snapshots": snapshot_metadata,
        "files": [f"{PRODUCT_PATH}/{name}.yaml" for name in SOURCE_FILES],
        "method": "Deep semantic endpoint diff plus added/removed lifecycle rows for transient-only intermediate symbols.",
        "limitations": [
            "Covers versioned wowless schema surfaces, not a historical FrameXML source tree.",
            "wowless ingestion commits may skip Blizzard client builds.",
            "CVar renames are represented as separate removals and additions.",
        ],
    }
    return register


def _render(register: dict) -> str:
    return json.dumps(register, indent=2, ensure_ascii=False) + "\n"


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wowless-dir", type=Path, default=WOWLESS_DIR)
    parser.add_argument("--output", type=Path, default=OUTPUT_PATH)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    register = generate_register(args.wowless_dir.expanduser())
    rendered = _render(register)
    if args.check:
        if not args.output.is_file() or args.output.read_text() != rendered:
            print(f"12.0.0 source register drift: {args.output}", file=sys.stderr)
            return 1
        print(f"12.0.0 source register is current: {args.output}")
        return 0
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered)
    print(
        f"Wrote {len(register['occurrences'])} occurrences to {args.output} "
        f"({register['direction_counts']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
