from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Iterable


class ReceiverKind(str, Enum):
    STRING = "string"
    ARRAY = "array"
    BYTES = "bytes"
    FREE = "free"


class NyraType(str, Enum):
    STRING = "string"
    I32 = "i32"
    I64 = "i64"
    F64 = "f64"
    BOOL = "bool"
    VOID = "void"
    PTR = "ptr"
    VEC_STR = "vec_str"
    BYTES = "bytes"
    ARRAY = "array"


@dataclass
class ArgSpec:
    name: str
    nyra_type: NyraType

    @classmethod
    def parse(cls, raw: str) -> ArgSpec:
        raw = raw.strip()
        if ":" not in raw:
            raise ValueError(
                f"expected name:type (e.g. suffix:string), got {raw!r}. "
                "Return types are configured in a later wizard step."
            )
        name, ty = raw.split(":", 1)
        name = name.strip()
        ty = ty.strip().lower()
        # tolerate plural / common aliases
        ty = {"strings": "string", "str": "string", "int": "i32", "integer": "i32", "pointer": "ptr"}.get(ty, ty)
        try:
            return cls(name=name, nyra_type=NyraType(ty))
        except ValueError as exc:
            raise ValueError(
                f"unknown Nyra type {ty!r} in {raw!r}. "
                f"Valid types: string, i32, i64, f64, bool, void, ptr, vec_str, bytes, array"
            ) from exc


@dataclass
class BuiltinSpec:
    receiver: ReceiverKind
    method: str
    args: list[ArgSpec] = field(default_factory=list)
    returns: NyraType = NyraType.STRING
    c_name: str | None = None
    rt_module: str | None = None
    borrows_receiver: bool = True
    owned_return: bool | None = None
    free_fn_alias: bool = True
    stable_abi: bool = False
    abi_since: str = "1.0.0"

    def __post_init__(self) -> None:
        self.method = self.method.strip()
        if not self.method:
            raise ValueError("method name is required")
        if self.c_name is None:
            self.c_name = default_c_name(self.receiver, self.method)
        if self.rt_module is None:
            self.rt_module = default_rt_module(self.receiver)
        if self.owned_return is None:
            self.owned_return = self.returns in (NyraType.STRING, NyraType.BYTES, NyraType.VEC_STR)

    @property
    def marker(self) -> str:
        return f"{self.method}:{self.receiver.value}"

    @property
    def wrapper_fn(self) -> str:
        parts = self.method.split("_")
        pascal = "".join(p[:1].upper() + p[1:] for p in parts if p)
        if self.receiver == ReceiverKind.STRING:
            return f"String_{pascal[0].lower()}{pascal[1:]}" if pascal else self.method
        if self.receiver == ReceiverKind.BYTES:
            return f"Bytes_{pascal[0].lower()}{pascal[1:]}" if pascal else self.method
        return self.method

    @classmethod
    def from_cli(
        cls,
        *,
        receiver: str,
        method: str,
        arg_specs: Iterable[str],
        returns: str,
        c_name: str | None,
        rt_module: str | None,
        borrows_receiver: bool,
        owned_return: bool | None,
        free_fn_alias: bool,
        stable_abi: bool,
        abi_since: str,
    ) -> BuiltinSpec:
        return cls(
            receiver=ReceiverKind(receiver.lower()),
            method=method,
            args=[ArgSpec.parse(a) for a in arg_specs],
            returns=NyraType(returns.lower()),
            c_name=c_name,
            rt_module=rt_module,
            borrows_receiver=borrows_receiver,
            owned_return=owned_return,
            free_fn_alias=free_fn_alias,
            stable_abi=stable_abi,
            abi_since=abi_since,
        )


def default_c_name(receiver: ReceiverKind, method: str) -> str:
    if receiver == ReceiverKind.STRING:
        if method == "split_once":
            return "str_before_sep"
        if method.startswith("str_"):
            return method
        return f"str_{method}"
    if receiver == ReceiverKind.BYTES:
        return f"bytes_{method}"
    return method.replace(".", "_")


def default_rt_module(receiver: ReceiverKind) -> str:
    return {
        ReceiverKind.STRING: "rt_strings.c",
        ReceiverKind.BYTES: "rt_bytes.c",
        ReceiverKind.ARRAY: "rt_array.c",
        ReceiverKind.FREE: "rt_strings.c",
    }[receiver]
