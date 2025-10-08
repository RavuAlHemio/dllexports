#!/usr/bin/env python3

import argparse
import enum
import json
import re
from typing import Any, Dict, FrozenSet, Iterable, List, Optional, Set, Union
import jinja2


TEMPLATE = """
{%- macro output_args(args) -%}
    {% for arg in args -%}
        {% if not loop.first %}, {% endif -%}
        {% if arg.direction.is_in %}[in] {% endif -%}
        {% if arg.direction.is_out %}[out] {% endif -%}
        {{ arg.arg_type.il_type(meta) }} '{{ arg.name }}'
    {% endfor -%}
{% endmacro -%}
{%- macro output_arg_attributes(args) -%}
    {% for arg in args -%}
        {% if arg.attributes -%}
            .param [{{ loop.index }}]
                {% if arg.has_attrib("const") -%}
                    .custom instance void [Windows.Win32.winmd]Windows.Win32.Foundation.Metadata.ConstAttribute::.ctor() = (
                        01 00 00 00
                    )
                {% endif %}
                {% if arg.has_attrib("com_out") -%}
                    .custom instance void [Windows.Win32.winmd]Windows.Win32.Foundation.Metadata.ComOutPtrAttribute::.ctor() = (
                        01 00 00 00
                    )
                {% endif %}
                {% if arg.count_in_arg is not none -%}
                    .custom instance void [Windows.Win32.winmd]Windows.Win32.Foundation.Metadata.NativeArrayInfoAttribute::.ctor() = (
                        01 00 01 00 53 06 0F 43 6F 75 6E 74 50 61 72 61   // ....S..CountPara
                        6D 49 6E 64 65 78 {{ arg.count_in_arg|hex_bytes_le(2) }}                           // mIndex..
                    )
                {% endif %}
        {% endif %}
    {% endfor %}
{% endmacro -%}

{%- for dll in imported_dlls -%}
.module extern '{{ dll }}'
{% endfor -%}

.assembly extern netstandard
{
    .publickeytoken = (
        cc 7b 13 ff cd 2d dd 51
    )
    .ver 2:1:0:0
}

.assembly {{ meta.name }}.winmd
{
    .ver {{ meta.version }}
}

.module {{ meta.name }}.winmd
.imagebase 0x00400000
.file alignment 0x00000200
.stackreserve 0x00100000
.subsystem 0x0003 // WindowsCui
.corflags 0x00000001 // ILOnly

{% for fptr in meta.func_ptrs -%}
.class public auto auto sealed beforefieldinit {{ meta.name }}.{{ fptr.name }}
    extends [netstandard]System.MulticastDelegate
{
    .custom instance void [netstandard]System.Runtime.InteropServices.UnmanagedFunctionPointerAttribute::.ctor(valuetype [netstandard]System.Runtime.InteropServices.CallingConvention) = (
        01 00 {{ fptr.call_conv_hex }} 00 00
    )

    .method public hidebysig specialname rtspecialname
        instance void .ctor (
            object 'object',
            native int 'method'
        ) runtime managed
    {
    }

    {# the actual pointer is described using a method named Invoke #}
    .method public hidebysig newslot virtual
        instance {{ fptr.return_type.il_type(meta) }} Invoke (
            {{ output_args(fptr.args) }}
        ) runtime managed
    {
        {{ output_arg_attributes(fptr.args) }}
    }
}
{% endfor %}{# meta.func_ptrs #}

{% if meta.funcs -%}
.class public auto auto abstract sealed beforefieldinit {{ meta.name }}.Apis
    extends [netstandard]System.Object
{
    {% for func in meta.funcs -%}
    .method public hidebysig pinvokeimpl("{{ func.dll }}" nomangle {{ func.call_conv }})
        {{ func.return_type.il_type(meta) }} {{ func.name }} (
            {{ output_args(func.args) }}
        ) cil managed
    {
        {{ output_arg_attributes(func.args) }}
    }
    {% endfor %}
}
{% endif %}{# meta.funcs #}

{% for iface in meta.interfaces -%}
.class interface public auto ansi abstract {{ meta.name }}.{{ iface.name }}
    implements {{ iface.base_type.il_type(meta) }}
{
    {# gghhiijj-kkll-mmnn-oopp-qqrrssttuuvv becomes 01 00 jj ii hh gg ll kk nn mm oo pp qq rr ss tt uu vv 00 00 #}
    {# 23170F69-40C1-278A-0000-00gg00vv0000 becomes 01 00 69 0F 17 23 C1 40 8A 27 00 00 00 gg 00 vv 00 00 00 00 #}
    .custom instance void [Windows.Win32.winmd]Windows.Win32.Foundation.Metadata.GuidAttribute::.ctor(uint32, uint16, uint16, uint8, uint8, uint8, uint8, uint8, uint8, uint8, uint8) = (
        01 00
        69 0F 17 23
        C1 40
        8A 27
        00 00
        00 {{ "{:02X}".format(iface.group) }} 00 {{ "{:02X}".format(iface.value) }} 00 00
        00 00
    )

    {% for meth in iface.methods %}
    .method public hidebysig newslot abstract virtual
        instance {{ meth.return_type.il_type(meta) }} {{ meth.name }} (
            {{ output_args(meth.args) }}
        ) cil managed
    {
        {{ output_arg_attributes(meth.args) }}
    }
    {% endfor %}
}
{% endfor %}{# meta.interfaces #}
"""

COUNT_IN_ARG_RE = re.compile("^ca([0-9]+)$")


class MetaType:
    def __init__(self, name: str, stars: int = 0) -> None:
        self.name: str = name
        self.stars: int = stars

    def il_type(self, meta: 'Metadata') -> str:
        def starless_il_type() -> str:
            remapped = {
                "BOOL": "valuetype [Windows.Win32.winmd]Windows.Win32.Foundation.BOOL",
                "BSTR": "valuetype [Windows.Win32.winmd]Windows.Win32.Foundation.BSTR",
                "FILETIME": "valuetype [Windows.Win32.winmd]Windows.Win32.Foundation.FILETIME",
                "GUID": "valuetype [netstandard]System.Guid",
                "HRESULT": "valuetype [Windows.Win32.winmd]Windows.Win32.Foundation.HRESULT",
                "IUnknown": "[Windows.Win32.winmd]Windows.Win32.System.Com.IUnknown",
                "PROPID": "uint32",
                "PROPVARIANT": "valuetype [Windows.Win32.winmd]Windows.Win32.System.Com.StructuredStorage.PROPVARIANT",
                "size_t": "native uint",
                "VARTYPE": "uint16",
            }.get(self.name, None)
            if remapped is not None:
                return remapped

            # "I[A-Z][a-z]..."
            is_com_interface = (
                self.name.startswith("I")
                and len(self.name) > 2
                and self.name[1].isascii() and self.name[1].isupper()
                and self.name[2].isascii() and self.name[2].islower()
            )
            if is_com_interface:
                return f"class {meta.name}.{self.name}"

            if any(fptr.name == self.name for fptr in meta.func_ptrs):
                return f"class {meta.name}.{self.name}"

            return self.name

        return starless_il_type() + (self.stars * "*")

class ArgumentDirection(enum.IntEnum):
    IN = 1
    OUT = 2
    INOUT = 3

    @property
    def is_in(self) -> bool:
        return self in (ArgumentDirection.IN, ArgumentDirection.INOUT)

    @property
    def is_out(self) -> bool:
        return self in (ArgumentDirection.OUT, ArgumentDirection.INOUT)

class ArgumentAttribute(enum.IntEnum):
    CONST = 1
    COM_OUT = 2

class Argument:
    def __init__(
        self, name: str, arg_type: MetaType, direction: ArgumentDirection,
        attributes: Iterable[ArgumentAttribute], count_in_arg: Optional[int] = None,
    ) -> None:
        self.name: str = name
        self.arg_type: MetaType = arg_type
        self.direction: ArgumentDirection = direction
        self.attributes: FrozenSet[ArgumentAttribute] = frozenset(attributes)
        self.count_in_arg: Optional[int] = None

    def has_attrib(self, attrib_name: str) -> bool:
        expected_value = getattr(ArgumentAttribute, attrib_name.upper())
        return expected_value in self.attributes


class FunctionLike:
    def __init__(self, name: str, return_type: MetaType) -> None:
        self.name: str = name
        self.return_type: MetaType = return_type
        self.args: List[Argument] = []

class Function(FunctionLike):
    def __init__(self, dll: str, name: str, return_type: MetaType, call_conv: Optional[str]) -> None:
        super().__init__(name, return_type)
        self.dll: str = dll
        self.call_conv: str = call_conv if call_conv is not None else "winapi"


class FunctionPointerType(FunctionLike):
    def __init__(self, name: str, return_type: MetaType, call_conv_int: int = 1) -> None:
        super().__init__(name, return_type)
        if call_conv_int < 0:
            raise ValueError("call_conv_int must be at least 0")
        self.call_conv_int: int = call_conv_int

    @property
    def call_conv_hex(self) -> str:
        bs = self.call_conv_int.to_bytes(4, "little")
        return " ".join(f"{b:02X}" for b in bs)


class Interface:
    def __init__(self, name: str, group: int, value: int, base_type: MetaType):
        if not (0 <= group <= 255):
            raise ValueError("group must be between 0 and 255")
        if not (0 <= value <= 255):
            raise ValueError("value must be between 0 and 255")

        self.name: str = name
        self.group: int = group
        self.value: int = value
        self.base_type: MetaType = base_type

        self.methods: List[Method] = []


class Method(FunctionLike):
    def __init__(self, name: str, return_type: MetaType) -> None:
        super().__init__(name, return_type)


class Metadata:
    def __init__(self, name: str, version: str) -> None:
        self.name: str = name
        self.version: str = version
        self.funcs: List[Function] = []
        self.func_ptrs: List[FunctionPointerType] = []
        self.interfaces: List[Interface] = []


def hex_bytes_le(number: int, byte_count: int) -> str:
    return " ".join(f"{b:02X}" for b in number.to_bytes(byte_count, "little"))


def run(json_path: str, il_path: str) -> None:
    meta: Optional[Metadata] = None
    dll: Optional[str] = None
    iface: Optional[Interface] = None
    func: Optional[FunctionLike] = None

    all_dlls: Set[str] = set()

    with open(json_path, "r", encoding="utf-8") as f:
        for raw_ln in f:
            # strip the newline
            ln = raw_ln.rstrip("\r\n")

            # strip comments
            hash_index = ln.find("#")
            if hash_index != -1:
                ln = ln[:hash_index].rstrip()

            # skip empty or whitespace-only lines
            if not ln.strip():
                continue

            pieces = ln.split("\t")

            # okay, what are we dealing with?
            if pieces[0] == "meta":
                if len(pieces) != 3:
                    raise ValueError("Usage: meta NAME VERSION")
                (name, version) = pieces[1:3]
                meta = Metadata(name, version)
                continue

            if meta is None:
                raise ValueError("first entry must be \"meta\"")

            if pieces[0] == "fptr":
                if len(pieces) != 4:
                    raise ValueError("Usage: fptr NAME RETTYPE RETSTARS")
                (name, ret_type, ret_stars_str) = pieces[1:4]
                ret_stars = int(ret_stars_str)
                WINAPI_INT = 1 # System.Runtime.InteropServices.CallingConvention
                func = FunctionPointerType(name, MetaType(ret_type, ret_stars), WINAPI_INT)
                meta.func_ptrs.append(func)
                continue

            if pieces[0] == "dll":
                if len(pieces) != 2:
                    raise ValueError("Usage: dll NAME")
                dll = pieces[1]
                all_dlls.add(pieces[1].lower())
                continue

            if pieces[0] == "fn":
                if len(pieces) != 4:
                    raise ValueError("Usage: fn NAME RETTYPE RETSTARS")
                if dll is None:
                    raise ValueError("\"fn\" entry without a previous \"dll\" entry")
                (name, ret_type, ret_stars_str) = pieces[1:4]
                ret_stars = int(ret_stars_str)
                func = Function(dll, name, MetaType(ret_type, ret_stars), None)
                meta.funcs.append(func)
                continue

            if pieces[0] == "arg":
                if len(pieces) not in (5, 6):
                    raise ValueError("Usage: arg DIRECTION NAME TYPE STARS [ATTRIBS]")
                if func is None:
                    raise ValueError("\"arg\" entry without a previous \"func\" or \"fptr\" entry")
                (direction_str, name, arg_type, arg_stars_str) = pieces[1:5]
                attribs_str = pieces[5] if len(pieces) > 5 else ""
                arg_stars = int(arg_stars_str)
                direction = {
                    "in": ArgumentDirection.IN,
                    "out": ArgumentDirection.OUT,
                    "inout": ArgumentDirection.INOUT,
                }[direction_str]
                count_in_arg = None
                if attribs_str.strip():
                    attrib_words = {
                        a.strip()
                        for a in attribs_str.split(" ")
                        if a.strip()
                    }

                    remove_us = set()
                    for word in attrib_words:
                        m = COUNT_IN_ARG_RE.match(word)
                        if m is None:
                            continue

                        count_in_arg = int(m.group(1))
                        remove_us.add(word)

                    for word in remove_us:
                        attrib_words.remove(word)

                    attribs: Set[ArgumentAttribute] = {
                        {
                            "const": ArgumentAttribute.CONST,
                            "com_out": ArgumentAttribute.COM_OUT,
                        }[a]
                        for a in attrib_words
                    }
                else:
                    attribs = set()

                arg = Argument(
                    name,
                    MetaType(arg_type, arg_stars),
                    direction,
                    attribs,
                    count_in_arg,
                )
                func.args.append(arg)
                continue

            if pieces[0] == "iface":
                if len(pieces) != 5:
                    raise ValueError("Usage: iface NAME GROUP VALUE BASETYPE")
                (name, group_str, value_str, base_type) = pieces[1:5]
                group, value = int(group_str), int(value_str)
                iface = Interface(name, group, value, MetaType(base_type, 0))
                meta.interfaces.append(iface)
                continue

            if pieces[0] == "meth":
                if len(pieces) != 4:
                    raise ValueError("Usage: meth NAME RETTYPE RETSTARS")
                if iface is None:
                    raise ValueError("\"meth\" entry without a previous \"iface\" entry")
                (name, ret_type, ret_stars_str) = pieces[1:4]
                ret_stars = int(ret_stars_str)
                func = Method(name, MetaType(ret_type, ret_stars))
                iface.methods.append(func)
                continue

            raise ValueError(f"unknown command {pieces[0]!r}")

    imported_dlls = sorted(all_dlls)

    env = jinja2.Environment(
        undefined=jinja2.StrictUndefined,
    )
    env.filters["hex_bytes_le"] = hex_bytes_le
    tpl = env.from_string(TEMPLATE)

    output = tpl.render(
        meta=meta,
        imported_dlls=imported_dlls,
    )

    with open(il_path, "w", encoding="utf-8") as f:
        f.write(output)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        dest="json_path",
    )
    parser.add_argument(
        dest="il_path",
    )
    args = parser.parse_args()

    run(args.json_path, args.il_path)


if __name__ == "__main__":
    main()
