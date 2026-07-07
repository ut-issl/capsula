"""Tests for capsula_client."""

from __future__ import annotations

import pytest

from capsula_client import (
    CapsulaClient,
    ComparisonOp,
    HookFilter,
    ParameterMatch,
    SearchRunsRequest,
    SortOrder,
)


# --- Model tests ---


class TestParameterMatch:
    def test_to_dict_file_and_parameter(self):
        pm = ParameterMatch(
            phase="pre",
            file="config/sat1/orbit.json",
            parameter="a",
            operator=ComparisonOp.GE,
            value=1.0,
        )
        assert pm.to_dict() == {
            "phase": "pre",
            "file": "config/sat1/orbit.json",
            "parameter": "a",
            "operator": "ge",
            "value": 1.0,
        }

    def test_to_dict_file_only_omits_triple(self):
        pm = ParameterMatch(phase="pre", file="config.json")
        d = pm.to_dict()
        assert d == {"phase": "pre", "file": "config.json"}
        assert "parameter" not in d
        assert "operator" not in d
        assert "value" not in d

    def test_to_dict_parameter_only_omits_file(self):
        pm = ParameterMatch(
            phase="pre",
            parameter="lr",
            operator=ComparisonOp.GE,
            value=0.01,
        )
        d = pm.to_dict()
        assert d == {
            "phase": "pre",
            "parameter": "lr",
            "operator": "ge",
            "value": 0.01,
        }
        assert "file" not in d

    def test_to_dict_nested_dot_path(self):
        pm = ParameterMatch(
            phase="pre",
            file="sat1/orbit.json",
            parameter="orbit.a",
            operator=ComparisonOp.GE,
            value=1.0,
        )
        d = pm.to_dict()
        assert d["file"] == "sat1/orbit.json"
        assert d["parameter"] == "orbit.a"
        assert d["operator"] == "ge"

    def test_to_dict_string_value(self):
        pm = ParameterMatch(
            phase="post",
            file="model.json",
            parameter="architecture",
            operator=ComparisonOp.EQ,
            value="transformer",
        )
        d = pm.to_dict()
        assert d["value"] == "transformer"
        assert d["phase"] == "post"
        assert d["file"] == "model.json"

    def test_to_dict_hook_index_alone(self):
        pm = ParameterMatch(phase="pre", hook_index=2)
        d = pm.to_dict()
        assert d == {"phase": "pre", "hook_index": 2}
        assert "file" not in d
        assert "parameter" not in d

    def test_to_dict_hook_index_composed_with_file_and_parameter(self):
        pm = ParameterMatch(
            phase="pre",
            file="config.json",
            hook_index=1,
            parameter="lr",
            operator=ComparisonOp.EQ,
            value=0.01,
        )
        d = pm.to_dict()
        assert d == {
            "phase": "pre",
            "file": "config.json",
            "hook_index": 1,
            "parameter": "lr",
            "operator": "eq",
            "value": 0.01,
        }

    def test_to_dict_hook_index_zero_is_included(self):
        # ``hook_index`` uses ``is not None`` (not a falsy check), so 0
        # must still be serialized — it is a legitimate array position.
        pm = ParameterMatch(phase="pre", hook_index=0)
        d = pm.to_dict()
        assert d["hook_index"] == 0

    def test_to_dict_default_hook_index_omitted(self):
        pm = ParameterMatch(phase="pre", file="c.json")
        d = pm.to_dict()
        assert "hook_index" not in d


class TestSearchRunsRequest:
    def test_minimal(self):
        req = SearchRunsRequest(vault="my-vault")
        d = req.to_dict()
        assert d["vault"] == "my-vault"
        assert d["order"] == "latest_first"
        assert "hook_filters" not in d
        assert "parameter_matches" not in d

    def test_with_parameter_matches(self):
        req = SearchRunsRequest(
            vault="thermal-sim",
            parameter_matches=[
                ParameterMatch(
                    phase="pre",
                    file="env.json",
                    parameter="solar_flux",
                    operator=ComparisonOp.GE,
                    value=1347.39,
                ),
                ParameterMatch(
                    phase="pre",
                    file="env.json",
                    parameter="solar_flux",
                    operator=ComparisonOp.LE,
                    value=1374.61,
                ),
            ],
            include=["hooks"],
            limit=1,
        )
        d = req.to_dict()
        assert len(d["parameter_matches"]) == 2
        assert d["parameter_matches"][0]["operator"] == "ge"
        assert d["parameter_matches"][0]["file"] == "env.json"
        assert d["parameter_matches"][1]["operator"] == "le"
        assert d["include"] == ["hooks"]
        assert d["limit"] == 1

    def test_with_hook_filter(self):
        req = SearchRunsRequest(
            hook_filters=[
                HookFilter("capture-git-repo", '$.sha ? (@ starts with "abc")'),
            ],
        )
        d = req.to_dict()
        assert len(d["hook_filters"]) == 1
        assert d["hook_filters"][0]["hook_id"] == "capture-git-repo"

    def test_omits_none_fields(self):
        req = SearchRunsRequest()
        d = req.to_dict()
        assert "vault" not in d
        assert "exit_code" not in d
        assert "from" not in d

    def test_order(self):
        req = SearchRunsRequest(order=SortOrder.OLDEST_FIRST)
        d = req.to_dict()
        assert d["order"] == "oldest_first"


# --- Client tests (with mocked HTTP) ---


class TestCapsulaClient:
    def test_list_vaults(self, httpx_mock):
        httpx_mock.add_response(
            url="https://capsula.test/api/v1/vaults",
            json={"status": "ok", "vaults": [{"name": "my-vault", "run_count": 5}]},
        )
        with CapsulaClient("https://capsula.test") as client:
            vaults = client.list_vaults()
        assert len(vaults) == 1
        assert vaults[0].name == "my-vault"
        assert vaults[0].run_count == 5

    def test_vault_exists(self, httpx_mock):
        httpx_mock.add_response(
            url="https://capsula.test/api/v1/vaults/my-vault",
            json={"status": "ok", "exists": True, "vault": {"name": "my-vault", "run_count": 3}},
        )
        with CapsulaClient("https://capsula.test") as client:
            vault = client.vault_exists("my-vault")
        assert vault is not None
        assert vault.name == "my-vault"

    def test_vault_not_exists(self, httpx_mock):
        httpx_mock.add_response(
            url="https://capsula.test/api/v1/vaults/nonexistent",
            json={"status": "ok", "exists": False, "vault": None},
        )
        with CapsulaClient("https://capsula.test") as client:
            vault = client.vault_exists("nonexistent")
        assert vault is None

    def test_search_runs(self, httpx_mock):
        httpx_mock.add_response(
            url="https://capsula.test/api/v1/runs/search",
            json={
                "status": "ok",
                "total": 1,
                "runs": [
                    {
                        "id": "abc123",
                        "name": "happy-river",
                        "timestamp": "2026-05-15T12:00:00Z",
                        "vault": "thermal-sim",
                        "command": "python sim.py",
                        "project_root": "/home/user/project",
                        "exit_code": 0,
                        "pre_run_hooks": [
                            {
                                "__meta": {
                                    "id": "capture-command",
                                    "success": True,
                                    "hook_index": 0,
                                },
                                "solar_flux": 1361.0,
                            }
                        ],
                        "post_run_hooks": [
                            {
                                "__meta": {
                                    "id": "capture-file",
                                    "success": True,
                                    "hook_index": 3,
                                },
                                "max_temperature": 85.3,
                            }
                        ],
                    }
                ],
            },
        )
        with CapsulaClient("https://capsula.test") as client:
            resp = client.search_runs(
                SearchRunsRequest(
                    vault="thermal-sim",
                    parameter_matches=[
                        ParameterMatch(
                            phase="pre",
                            file="env.json",
                            parameter="solar_flux",
                            operator=ComparisonOp.GE,
                            value=1347.39,
                        ),
                        ParameterMatch(
                            phase="pre",
                            file="env.json",
                            parameter="solar_flux",
                            operator=ComparisonOp.LE,
                            value=1374.61,
                        ),
                    ],
                    include=["hooks"],
                    limit=1,
                )
            )
        assert resp.total == 1
        assert resp.runs[0].id == "abc123"
        assert resp.runs[0].pre_run_hooks is not None
        assert resp.runs[0].pre_run_hooks[0].output["solar_flux"] == 1361.0
        assert resp.runs[0].pre_run_hooks[0].hook_index == 0
        assert resp.runs[0].post_run_hooks is not None
        assert resp.runs[0].post_run_hooks[0].output["max_temperature"] == 85.3
        assert resp.runs[0].post_run_hooks[0].hook_index == 3

    def test_search_runs_missing_hook_index_is_none(self, httpx_mock):
        # Older server responses (before hook_index was exposed) omit the
        # field in ``__meta``. The client should tolerate this by leaving
        # ``HookOutput.hook_index`` as ``None`` rather than raising.
        httpx_mock.add_response(
            url="https://capsula.test/api/v1/runs/search",
            json={
                "status": "ok",
                "total": 1,
                "runs": [
                    {
                        "id": "legacy001",
                        "name": "legacy-response",
                        "timestamp": "2026-05-15T12:00:00Z",
                        "vault": "v1",
                        "command": "true",
                        "project_root": "/tmp/p",
                        "exit_code": 0,
                        "pre_run_hooks": [
                            {
                                "__meta": {"id": "capture-command", "success": True},
                                "solar_flux": 1361.0,
                            }
                        ],
                    }
                ],
            },
        )
        with CapsulaClient("https://capsula.test") as client:
            resp = client.search_runs(SearchRunsRequest(vault="v1"))
        assert resp.runs[0].pre_run_hooks is not None
        assert resp.runs[0].pre_run_hooks[0].hook_index is None

    def test_search_runs_empty(self, httpx_mock):
        httpx_mock.add_response(
            url="https://capsula.test/api/v1/runs/search",
            json={"status": "ok", "total": 0, "runs": []},
        )
        with CapsulaClient("https://capsula.test") as client:
            resp = client.search_runs(SearchRunsRequest(vault="nonexistent"))
        assert resp.total == 0
        assert resp.runs == []

    def test_server_error(self, httpx_mock):
        httpx_mock.add_response(
            url="https://capsula.test/api/v1/vaults",
            status_code=500,
            json={"status": "error", "error": "internal server error"},
        )
        with CapsulaClient("https://capsula.test") as client:
            with pytest.raises(Exception, match="500"):
                client.list_vaults()
