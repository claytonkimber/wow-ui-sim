from __future__ import annotations

import copy
import re
import sys
import unittest
from collections import Counter
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))

from tools.gen_patch_12_0_0_register import (  # noqa: E402
    build_register,
    diff_endpoint_symbols,
    extract_symbols,
    merge_docs_lie_metadata,
)


LUA_PATH = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$")


class Patch1200RegisterTests(unittest.TestCase):
    def snapshot(self) -> dict:
        return {
            "apis": [
                {"namespace": "C_Test", "name": "GetValue", "returns": ["number"]},
                {"namespace": "C_Test", "name": "SetValue", "arguments": ["number"]},
            ],
            "globals": [{"name": "GetGlobal"}],
            "constants": [{"group": "TestConsts", "name": "CONST_FOO", "value": 7}],
            "enums": [{"namespace": "Test", "name": "Value", "value": 1}],
            "cvars": [{"name": "testCVar", "default": "0"}],
            "events": [{"name": "EVENT_REMOVED"}, {"name": "EVENT_KEPT"}],
            "structures": [
                {
                    "name": "TestStruct",
                    "fields": [{"name": "id", "type": "number"}, {"name": "label", "type": "string"}],
                }
            ],
            "uiobjects": [
                {
                    "name": "Button",
                    "methods": [{"name": "SetFoo"}, {"name": "GetFoo"}],
                }
            ],
            "luaobjects": [
                {"name": "TestObject", "methods": [{"name": "GetFoo"}]}
            ],
            "typedefs": [{"name": "TestID", "type": "number"}],
            "script_objects": [{"name": "TestScriptObject"}],
        }

    def test_extracts_every_snapshot_category_and_nested_member(self) -> None:
        records = extract_symbols(self.snapshot())
        by_path = {record["path"]: record for record in records}

        expected_categories = {
            "C_Test.GetValue": "api",
            "C_Test.SetValue": "api",
            "GetGlobal": "global",
            "Constants.TestConsts.CONST_FOO": "constant",
            "Enum.Test.Value": "enum",
            "testCVar": "cvar",
            "EVENT_REMOVED": "event",
            "EVENT_KEPT": "event",
            "TestStruct": "structure",
            "TestStruct.id": "structure-field",
            "TestStruct.label": "structure-field",
            "Button": "uiobject",
            "Button.SetFoo": "uiobject-method",
            "Button.GetFoo": "uiobject-method",
            "TestObject": "luaobject",
            "TestObject.GetFoo": "luaobject-method",
            "typedef.TestID": "typedef",
            "script_object.TestScriptObject": "script-object",
        }

        self.assertEqual(set(by_path), set(expected_categories))
        for path, category in expected_categories.items():
            self.assertEqual(by_path[path]["category"], category)

        counts = Counter(record["category"] for record in records)
        self.assertEqual(counts["api"], 2)
        self.assertEqual(counts["structure-field"], 2)
        self.assertEqual(counts["uiobject-method"], 2)
        self.assertEqual(counts["luaobject-method"], 1)
        self.assertEqual(sum(counts.values()), len(expected_categories))

    def test_extracts_raw_wowless_yaml_shapes_and_merges_docs_lies(self) -> None:
        snapshot = {
            "apis": {
                "C_Test.GetValue": {"outputs": [{"name": "value", "type": "number"}]},
            },
            "globals": {
                "Constants": {"TestConsts": {"CONST_FOO": 7}},
                "Enum": {"Test": {"Value": 1}},
                "GetGlobal": None,
            },
            "cvars": {"testCVar": "0"},
            "events": {"EVENT_KEPT": {"payload": []}},
            "structures": {
                "TestStruct": {
                    "fields": {"id": {"type": "number"}},
                }
            },
            "uiobjects": {
                "Button": {
                    "inherits": "Frame",
                    "methods": {"SetFoo": {"inputs": None, "outputs": None}},
                }
            },
            "luaobjects": {
                "TestObject": {
                    "methods": {"GetFoo": {"inputs": None, "outputs": None}},
                }
            },
            "docs": {
                "typedefs": {"TestID": "number"},
                "script_objects": {"TestScriptObject": {"type": "TestObject"}},
                "lies": {
                    "apis": {"C_Test.GetValue": {"claim": "returns string"}},
                    "uiobjects": {"Button": {"SetFoo": {"claim": "not available"}}},
                    "extra_events": {"TRANSIENT_DOC_EVENT": None},
                },
                "ignore_script_object_methods": {"Button": ["SetFoo"]},
                "uiobject_method_reassignments": {"Button": {"SetFoo": "Frame.SetFoo"}},
            },
        }

        records = extract_symbols(snapshot)
        by_path = {record["path"]: record for record in records}

        self.assertEqual(by_path["testCVar"]["category"], "cvar")
        self.assertEqual(by_path["Constants.TestConsts.CONST_FOO"]["category"], "constant")
        self.assertEqual(by_path["typedef.TestID"]["category"], "typedef")
        self.assertEqual(
            by_path["C_Test.GetValue"]["metadata"]["docs_lie"],
            {"claim": "returns string"},
        )
        self.assertEqual(
            by_path["Button.SetFoo"]["metadata"]["docs_lie"],
            {"claim": "not available"},
        )
        self.assertEqual(
            by_path["Button.SetFoo"]["metadata"]["ignored_script_object_method"],
            True,
        )
        self.assertEqual(
            by_path["Button.SetFoo"]["metadata"]["uiobject_method_reassignment"],
            "Frame.SetFoo",
        )
        self.assertEqual(
            by_path["docs.extra_events.TRANSIENT_DOC_EVENT"]["category"],
            "docs-extra-event",
        )

    def test_empty_raw_yaml_surface_is_treated_as_an_empty_mapping(self) -> None:
        snapshot = {
            "apis": {},
            "globals": {},
            "cvars": {},
            "events": {},
            "structures": {},
            "uiobjects": {},
            "luaobjects": None,
            "docs": {},
        }

        self.assertEqual(extract_symbols(snapshot), [])

    def test_docs_lie_metadata_merges_into_existing_symbols_without_duplicates(self) -> None:
        records = extract_symbols(self.snapshot())
        docs_lies = {
            "C_Test.GetValue": {"source": "docs", "claim": "returns string"},
            "EVENT_KEPT": {"source": "docs", "claim": "removed"},
            "Button.SetFoo": {"source": "docs", "claim": "not available"},
        }

        merged = merge_docs_lie_metadata(records, docs_lies)
        by_path = {record["path"]: record for record in merged}

        self.assertEqual(len(merged), len(records))
        self.assertEqual(len(by_path), len(merged))
        self.assertEqual(
            by_path["C_Test.GetValue"]["metadata"]["docs_lie"],
            docs_lies["C_Test.GetValue"],
        )
        self.assertEqual(
            by_path["EVENT_KEPT"]["metadata"]["docs_lie"],
            docs_lies["EVENT_KEPT"],
        )
        self.assertEqual(
            by_path["Button.SetFoo"]["metadata"]["docs_lie"],
            docs_lies["Button.SetFoo"],
        )

    def test_endpoint_diff_is_deterministic_and_reports_added_changed_removed(self) -> None:
        before = self.snapshot()
        after = copy.deepcopy(before)
        after["apis"][0]["returns"] = ["string"]
        after["globals"].append({"name": "GetNewGlobal"})
        after["events"] = [event for event in after["events"] if event["name"] != "EVENT_REMOVED"]

        first = diff_endpoint_symbols(before, after)
        shuffled_before = self.reverse_snapshot_lists(before)
        shuffled_after = self.reverse_snapshot_lists(after)
        second = diff_endpoint_symbols(shuffled_before, shuffled_after)

        self.assertEqual(first, second)
        changes = {(record["direction"], record["symbol"]) for record in first}
        self.assertEqual(
            changes,
            {
                ("added", "GetNewGlobal"),
                ("changed", "C_Test.GetValue"),
                ("removed", "EVENT_REMOVED"),
            },
        )
        self.assertEqual(
            [(record["direction"], record["symbol"]) for record in first],
            [
                ("added", "GetNewGlobal"),
                ("changed", "C_Test.GetValue"),
                ("removed", "EVENT_REMOVED"),
            ],
        )

        by_change = {(record["direction"], record["symbol"]): record for record in first}
        added = by_change[("added", "GetNewGlobal")]
        self.assertEqual(set(added), {"direction", "category", "symbol", "detail", "after"})
        self.assertEqual(
            added["after"],
            {"category": "global", "value": {}, "metadata": {}},
        )

        removed = by_change[("removed", "EVENT_REMOVED")]
        self.assertEqual(set(removed), {"direction", "category", "symbol", "detail", "before"})
        self.assertEqual(
            removed["before"],
            {"category": "event", "value": {}, "metadata": {}},
        )

        changed = by_change[("changed", "C_Test.GetValue")]
        self.assertEqual(
            set(changed),
            {"direction", "category", "symbol", "detail", "before", "after"},
        )
        self.assertEqual(
            changed["before"],
            {"category": "api", "value": {"returns": ["number"]}, "metadata": {}},
        )
        self.assertEqual(
            changed["after"],
            {"category": "api", "value": {"returns": ["string"]}, "metadata": {}},
        )

    def test_transient_symbol_emits_added_and_removed_with_lifecycle_detail(self) -> None:
        before = self.snapshot()
        after = self.snapshot()
        intermediate = self.snapshot()
        intermediate["apis"].append(
            {"namespace": "C_Transient", "name": "DuringPatch", "returns": ["boolean"]}
        )

        occurrences = diff_endpoint_symbols(
            before,
            after,
            intermediate_snapshots=[intermediate],
        )
        transient = [
            record for record in occurrences if record["symbol"] == "C_Transient.DuringPatch"
        ]

        self.assertEqual([record["direction"] for record in transient], ["added", "removed"])
        self.assertTrue(all("transient" in record["detail"].lower() for record in transient))
        self.assertTrue(all("intermediate" in record["detail"].lower() for record in transient))

        transient_payload = {
            "category": "api",
            "value": {"returns": ["boolean"]},
            "metadata": {},
        }
        transient_added, transient_removed = transient
        self.assertEqual(
            set(transient_added),
            {"direction", "category", "symbol", "detail", "after"},
        )
        self.assertEqual(transient_added["after"], transient_payload)
        self.assertEqual(
            set(transient_removed),
            {"direction", "category", "symbol", "detail", "before"},
        )
        self.assertEqual(transient_removed["before"], transient_payload)

    def test_register_paths_order_and_category_counts_match_occurrences(self) -> None:
        before = self.snapshot()
        after = copy.deepcopy(before)
        after["globals"].append({"name": "GetNewGlobal"})
        after["events"] = [event for event in after["events"] if event["name"] != "EVENT_REMOVED"]
        intermediate = self.snapshot()
        intermediate["apis"].append(
            {"namespace": "C_Transient", "name": "DuringPatch", "returns": ["boolean"]}
        )
        docs_lies = {
            "C_Test.GetValue": {"source": "docs", "claim": "returns string"},
            "EVENT_KEPT": {"source": "docs", "claim": "removed"},
            "Button.SetFoo": {"source": "docs", "claim": "not available"},
        }

        register = build_register(
            before,
            after,
            intermediate_snapshots=[intermediate],
            docs_lies=docs_lies,
        )
        occurrences = register["occurrences"]

        self.assertTrue(all(LUA_PATH.fullmatch(record["symbol"]) for record in occurrences))
        self.assertEqual(
            register["category_counts"],
            dict(Counter(record["category"] for record in occurrences)),
        )
        self.assertEqual(
            [(record["direction"], record["symbol"]) for record in occurrences],
            sorted(
                (record["direction"], record["symbol"])
                for record in occurrences
            ),
        )
        self.assertEqual(
            len({(record["symbol"], record["direction"]) for record in occurrences}),
            len(occurrences),
        )
        self.assertEqual(
            register["category_counts"]["global"],
            1,
        )
        self.assertEqual(
            register["category_counts"]["event"],
            1,
        )

    @staticmethod
    def reverse_snapshot_lists(snapshot: dict) -> dict:
        reversed_snapshot = copy.deepcopy(snapshot)
        for value in reversed_snapshot.values():
            if isinstance(value, list):
                value.reverse()
                for item in value:
                    if isinstance(item, dict):
                        for nested in item.values():
                            if isinstance(nested, list):
                                nested.reverse()
        return reversed_snapshot


if __name__ == "__main__":
    unittest.main()
