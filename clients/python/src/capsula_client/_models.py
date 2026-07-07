"""Data models mirroring capsula-api-types (Rust) for the Python client."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class ComparisonOp(str, Enum):
    """Comparison operator for parameter matching."""

    EQ = "eq"
    NE = "ne"
    GT = "gt"
    GE = "ge"
    LT = "lt"
    LE = "le"


class SortOrder(str, Enum):
    """Sort order for search results."""

    LATEST_FIRST = "latest_first"
    OLDEST_FIRST = "oldest_first"


@dataclass(frozen=True, slots=True)
class VaultInfo:
    """Information about a vault."""

    name: str
    run_count: int


@dataclass(frozen=True, slots=True)
class HookFilter:
    """A filter condition on a hook's output using JSONPath."""

    hook_id: str
    output_filter: str
    config_filter: str | None = None


@dataclass(frozen=True, slots=True)
class ParameterMatch:
    """Structured filter for parameter-capturing hooks (``capture-json``, ``capture-toml``, ...).

    Targets ``run_outputs`` rows whose ``output`` JSON has the shape
    ``{"content": <parsed JSON>}`` and whose ``__meta.config.path`` carries
    the source file path. The server picks matching rows structurally (by
    the presence of the ``content`` field), so the filter is decoupled from
    any specific ``hook_id``.

    Constraints (validated server-side; the client does not raise):

    - At least one of ``file`` / ``hook_index`` / ``parameter`` must be
      specified.
    - ``parameter``, ``operator``, and ``value`` are an all-or-nothing
      triple — supply all three or none.
    - Specifying ``parameter`` without either ``file`` or ``hook_index``
      causes the server to log a warning, since the match will scan every
      parameter-capturing row of the run.

    Example:
        ``ParameterMatch("pre", file="config/sat1/orbit.json",
        parameter="a", operator=ComparisonOp.GE, value=1.0)``
        generates ``$.content.a ? (@ >= 1.0)`` against the row whose
        ``config.path == "config/sat1/orbit.json"``.

    Use ``hook_index`` to pin the match to a specific 0-based position in
    the phase's array — useful when several parameter-capturing hooks in
    ``capsula.toml`` share the same ``file`` / ``hook_id`` and would
    otherwise be indistinguishable.

    For querying outputs of non-parameter hooks (``capture-env``,
    ``capture-command``, ...), use :class:`HookFilter` with a raw JSONPath
    expression instead.
    """

    phase: str
    file: str | None = None
    hook_index: int | None = None
    parameter: str | None = None
    operator: ComparisonOp | None = None
    value: Any | None = None  # number, string, or bool

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"phase": self.phase}
        if self.file is not None:
            d["file"] = self.file
        if self.hook_index is not None:
            d["hook_index"] = self.hook_index
        if self.parameter is not None:
            d["parameter"] = self.parameter
        if self.operator is not None:
            d["operator"] = self.operator.value
        if self.value is not None:
            d["value"] = self.value
        return d


@dataclass(slots=True)
class SearchRunsRequest:
    """Request body for POST /api/v1/runs/search."""

    vault: str | None = None
    from_: str | None = None
    to: str | None = None
    exit_code: int | None = None
    success: bool | None = None
    hook_filters: list[HookFilter] = field(default_factory=list)
    parameter_matches: list[ParameterMatch] = field(default_factory=list)
    include: list[str] = field(default_factory=list)
    order: SortOrder = SortOrder.LATEST_FIRST
    limit: int | None = None
    offset: int | None = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {}
        if self.vault is not None:
            d["vault"] = self.vault
        if self.from_ is not None:
            d["from"] = self.from_
        if self.to is not None:
            d["to"] = self.to
        if self.exit_code is not None:
            d["exit_code"] = self.exit_code
        if self.success is not None:
            d["success"] = self.success
        if self.hook_filters:
            d["hook_filters"] = [
                {
                    "hook_id": f.hook_id,
                    "output_filter": f.output_filter,
                    **({"config_filter": f.config_filter} if f.config_filter else {}),
                }
                for f in self.hook_filters
            ]
        if self.parameter_matches:
            d["parameter_matches"] = [pm.to_dict() for pm in self.parameter_matches]
        if self.include:
            d["include"] = self.include
        d["order"] = self.order.value
        if self.limit is not None:
            d["limit"] = self.limit
        if self.offset is not None:
            d["offset"] = self.offset
        return d


@dataclass(frozen=True, slots=True)
class HookOutput:
    """A hook's output from a run.

    ``hook_index`` is the 0-based position of this hook in the phase's array
    (``pre_run`` / ``post_run`` in ``capsula.toml``). It is populated from
    ``__meta.hook_index`` in the server response; older server responses
    that do not include it leave the field as ``None``.
    """

    hook_id: str
    output: dict[str, Any]
    success: bool
    config: dict[str, Any] | None = None
    error: str | None = None
    hook_index: int | None = None


@dataclass(frozen=True, slots=True)
class FileInfo:
    """File information in search results."""

    path: str
    size: int
    url: str
    hash: str | None = None


@dataclass(frozen=True, slots=True)
class SearchRunResult:
    """A single run in search results."""

    id: str
    name: str
    timestamp: str
    vault: str
    command: str
    project_root: str
    exit_code: int | None = None
    duration_ms: int | None = None
    stdout: str | None = None
    stderr: str | None = None
    files: list[FileInfo] | None = None
    pre_run_hooks: list[HookOutput] | None = None
    post_run_hooks: list[HookOutput] | None = None


@dataclass(frozen=True, slots=True)
class SearchRunsResponse:
    """Response from POST /api/v1/runs/search."""

    status: str
    total: int
    runs: list[SearchRunResult]
