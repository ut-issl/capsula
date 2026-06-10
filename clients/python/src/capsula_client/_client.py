"""Capsula server HTTP client."""

from __future__ import annotations

from typing import Any

import httpx

from capsula_client._models import (
    FileInfo,
    HookOutput,
    SearchRunResult,
    SearchRunsRequest,
    SearchRunsResponse,
    VaultInfo,
)


class CapsulaClientError(Exception):
    """Error communicating with the Capsula server."""


class CapsulaClient:
    """Python client for the Capsula server API.

    Usage::

        client = CapsulaClient("https://capsula.example.com")
        vaults = client.list_vaults()
        results = client.search_runs(SearchRunsRequest(vault="my-vault"))
    """

    def __init__(self, base_url: str, *, timeout: float = 30.0) -> None:
        self._base_url = base_url.rstrip("/")
        self._client = httpx.Client(base_url=self._base_url, timeout=timeout)

    def close(self) -> None:
        """Close the underlying HTTP client."""
        self._client.close()

    def __enter__(self) -> CapsulaClient:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()

    # --- Vaults ---

    def list_vaults(self) -> list[VaultInfo]:
        """List all vaults on the server."""
        resp = self._request("GET", "/api/v1/vaults")
        return [VaultInfo(name=v["name"], run_count=v["run_count"]) for v in resp["vaults"]]

    def vault_exists(self, vault_name: str) -> VaultInfo | None:
        """Check if a vault exists. Returns VaultInfo or None."""
        resp = self._request("GET", f"/api/v1/vaults/{vault_name}")
        if resp.get("exists"):
            v = resp["vault"]
            return VaultInfo(name=v["name"], run_count=v["run_count"])
        return None

    # --- Runs ---

    def search_runs(self, request: SearchRunsRequest) -> SearchRunsResponse:
        """Search for runs matching the given criteria."""
        resp = self._request("POST", "/api/v1/runs/search", json=request.to_dict())
        runs = [_parse_run(r) for r in resp.get("runs", [])]
        return SearchRunsResponse(
            status=resp.get("status", "ok"),
            total=resp.get("total", len(runs)),
            runs=runs,
        )

    def get_run(self, run_id: str) -> dict[str, Any]:
        """Get details of a specific run."""
        return self._request("GET", f"/api/v1/runs/{run_id}")

    # --- Internal ---

    def _request(self, method: str, path: str, **kwargs: Any) -> dict[str, Any]:
        """Make an HTTP request and return the parsed JSON response."""
        try:
            resp = self._client.request(method, path, **kwargs)
            resp.raise_for_status()
        except httpx.HTTPStatusError as e:
            raise CapsulaClientError(
                f"Server returned {e.response.status_code}: {e.response.text}"
            ) from e
        except httpx.HTTPError as e:
            raise CapsulaClientError(f"HTTP request failed: {e}") from e
        return resp.json()


def _parse_run(data: dict[str, Any]) -> SearchRunResult:
    """Parse a run from the server response."""
    return SearchRunResult(
        id=data["id"],
        name=data["name"],
        timestamp=data["timestamp"],
        vault=data["vault"],
        command=data["command"],
        project_root=data["project_root"],
        exit_code=data.get("exit_code"),
        duration_ms=data.get("duration_ms"),
        stdout=data.get("stdout"),
        stderr=data.get("stderr"),
        files=_parse_files(data.get("files")) if data.get("files") is not None else None,
        pre_run_hooks=_parse_hooks(data.get("pre_run_hooks")) if data.get("pre_run_hooks") is not None else None,
        post_run_hooks=_parse_hooks(data.get("post_run_hooks")) if data.get("post_run_hooks") is not None else None,
    )


def _parse_hooks(hooks: list[dict[str, Any]]) -> list[HookOutput]:
    """Parse hook outputs from the server response."""
    result = []
    for h in hooks:
        meta = h.get("__meta", {})
        output = {k: v for k, v in h.items() if k != "__meta"}
        result.append(
            HookOutput(
                hook_id=meta.get("id", ""),
                output=output,
                success=meta.get("success", True),
                config=meta.get("config"),
                error=meta.get("error"),
            )
        )
    return result


def _parse_files(files: list[dict[str, Any]]) -> list[FileInfo]:
    """Parse file info from the server response."""
    return [
        FileInfo(
            path=f["path"],
            size=f["size"],
            url=f["url"],
            hash=f.get("hash"),
        )
        for f in files
    ]
