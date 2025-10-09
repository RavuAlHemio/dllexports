#!/usr/bin/env python3

import argparse
import enum
import re
from typing import Dict, FrozenSet, Iterable, List, Optional, Set
import jinja2


TEMPLATE = """
{%- macro output_args(args) -%}
    {% for arg in args -%}
        {% if not loop.first %}, {% endif -%}
        {% if arg.direction.is_in %}[in] {% endif -%}
        {% if arg.direction.is_out %}[out] {% endif -%}
        {% if arg.optional %}[opt] {% endif -%}
        {{ arg.arg_type.il_type(meta) }} '{{ arg.name }}'
    {% endfor -%}
{% endmacro -%}
{%- macro output_arg_attributes(args) -%}
    {% for arg in args -%}
        {% if arg.attributes or arg.count_in_arg is not none or arg.arg_type.base_enum is not none -%}
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
                        01 00 01 00
                        53 06
                        0F 43 6F 75 6E 74 50 61 72 61 6D 49 6E 64 65 78 // length prefix plus "CountParamIndex"
                        {{ arg.count_in_arg|hex_bytes_le(2) }}
                    )
                {% elif arg.const_count is not none -%}
                    .custom instance void [Windows.Win32.winmd]Windows.Win32.Foundation.Metadata.NativeArrayInfoAttribute::.ctor() = (
                        01 00 01 00
                        53 08
                        0A 43 6F 75 6E 74 43 6F 6E 73 74 // length prefix plus "CountConst"
                        {{ arg.const_count|hex_bytes_le(4) }}
                    )
                {% endif %}
                {% if arg.arg_type.base_enum is not none -%}
                    .custom instance void [Windows.Win32.winmd]Windows.Win32.Foundation.Metadata.AssociatedEnumAttribute::.ctor() = (
                        01 00
                        {{ arg.arg_type.base_enum|pascal_str_hex_bytes }}
                        00 00
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
.assembly extern Windows.Win32.winmd
{
	.ver 0:0:0:0
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

{% for enum in meta.name_to_enum.values() -%}
.class public auto ansi sealed {{ meta.name }}.{{ enum.name }}
       extends [netstandard]System.Enum
{
  .field public specialname rtspecialname {{ enum.base_type.il_type(meta) }} value__
  {% for variant in enum.name_to_variant.values() %}
  .field public static literal valuetype {{ meta.name }}.{{ enum.name }} {{ variant.name }} = {{ enum.base_type.il_type(meta) }}({{ variant.value }})
  {% endfor %}
}
{% endfor %}{# meta.name_to_enum #}
"""

CONST_COUNT_RE = re.compile("^cc([0-9]+)$")
COUNT_IN_ARG_RE = re.compile("^ca([0-9]+)$")


class MetaType:
    def __init__(self, name: str, stars: int = 0) -> None:
        if stars < 0:
            raise ValueError("stars must be at least 0")

        self.name: str = name
        self.stars: int = stars
        self.base_enum: Option[str] = None

    def enrich_base_enum(self, meta: 'Metadata') -> None:
        if self.stars != 0:
            return

        ilt = self.il_type(meta)
        en = meta.name_to_enum.get(ilt, None)
        if en is not None:
            self.name = en.base_type.name
            self.stars = en.base_type.stars
            self.base_enum = en.name

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
        self, name: str, arg_type: MetaType, direction: ArgumentDirection, optional: bool,
        attributes: Iterable[ArgumentAttribute], count_in_arg: Optional[int] = None,
        const_count: Optional[int] = None,
    ) -> None:
        if count_in_arg is not None and const_count is not None:
            raise ValueError("count_in_arg and const_count must not be set simultaneously")

        self.name: str = name
        self.arg_type: MetaType = arg_type
        self.direction: ArgumentDirection = direction
        self.optional: bool = optional
        self.attributes: FrozenSet[ArgumentAttribute] = frozenset(attributes)
        self.count_in_arg: Optional[int] = count_in_arg
        self.const_count: Optional[int] = const_count

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
    pass


class EnumVariant:
    def __init__(self, name: str, value: int):
        self.name: str = name
        self.value: int = value


class Enumeration:
    def __init__(self, name: str, base_type: MetaType) -> None:
        if base_type.stars != 0:
            raise ValueError("enumeration base type must have 0 stars")

        self.name: str = name
        self.base_type: MetaType = base_type
        self.name_to_variant: Dict[str, EnumVariant] = {}


class Metadata:
    def __init__(self, name: str, version: str) -> None:
        self.name: str = name
        self.version: str = version
        self.funcs: List[Function] = []
        self.func_ptrs: List[FunctionPointerType] = []
        self.interfaces: List[Interface] = []
        self.name_to_enum: Dict[str, Enumeration] = {}

def pascal_str_hex_bytes(text: str) -> str:
    length = len(text)
    # not sure if there's a multibyte encoding where the top bit is set, so cut off at 127
    if length > 127:
        raise ValueError("text too long for Pascal string")
    text_hex = " ".join(f"{b:02X}" for b in text.encode("utf-8"))
    return f"{length:02X} {text_hex}"

def hex_bytes_le(number: int, byte_count: int) -> str:
    return " ".join(f"{b:02X}" for b in number.to_bytes(byte_count, "little"))


class CollectorState:
    def __init__(self) -> None:
        self.meta: Optional[Metadata] = None
        self.dll: Optional[str] = None
        self.iface: Optional[Interface] = None
        self.func: Optional[FunctionLike] = None
        self.enum: Optional[Enumeration] = None
        self.all_dlls: Set[str] = set()

    def collect_path(self, txt_path: str) -> None:
        with open(txt_path, "r", encoding="utf-8") as f:
            for (line_index, raw_ln) in enumerate(f):
                line_number = line_index + 1

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
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: meta NAME VERSION")
                    (name, version) = pieces[1:3]
                    self.meta = Metadata(name, version)
                    continue

                if self.meta is None:
                    raise ValueError(f"file {txt_path} line {line_number}: first entry must be \"meta\"")

                if pieces[0] == "fptr":
                    if len(pieces) != 4:
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: fptr NAME RETTYPE RETSTARS")
                    (name, ret_type, ret_stars_str) = pieces[1:4]
                    ret_stars = int(ret_stars_str)
                    WINAPI_INT = 1 # System.Runtime.InteropServices.CallingConvention
                    self.func = FunctionPointerType(name, MetaType(ret_type, ret_stars), WINAPI_INT)
                    self.func.return_type.enrich_base_enum(self.meta)
                    self.meta.func_ptrs.append(self.func)
                    continue

                if pieces[0] == "dll":
                    if len(pieces) != 2:
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: dll NAME")
                    self.dll = pieces[1]
                    self.all_dlls.add(pieces[1].lower())
                    continue

                if pieces[0] == "fn":
                    if len(pieces) != 4:
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: fn NAME RETTYPE RETSTARS")
                    if self.dll is None:
                        raise ValueError(f"file {txt_path} line {line_number}: \"fn\" entry without a previous \"dll\" entry")
                    (name, ret_type, ret_stars_str) = pieces[1:4]
                    ret_stars = int(ret_stars_str)
                    self.func = Function(self.dll, name, MetaType(ret_type, ret_stars), None)
                    self.func.return_type.enrich_base_enum(self.meta)
                    self.meta.funcs.append(self.func)
                    continue

                if pieces[0] in ("arg", "oarg"):
                    if len(pieces) not in (5, 6):
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: arg|oarg DIRECTION NAME TYPE STARS [ATTRIBS] (oarg = optional argument)")
                    if self.func is None:
                        raise ValueError(f"file {txt_path} line {line_number}: \"arg\" entry without a previous \"func\" or \"fptr\" entry")
                    (direction_str, name, arg_type, arg_stars_str) = pieces[1:5]
                    attribs_str = pieces[5] if len(pieces) > 5 else ""
                    arg_stars = int(arg_stars_str)
                    direction = {
                        "in": ArgumentDirection.IN,
                        "out": ArgumentDirection.OUT,
                        "inout": ArgumentDirection.INOUT,
                    }[direction_str]
                    const_count = None
                    count_in_arg = None
                    if attribs_str.strip():
                        attrib_words = {
                            a.strip()
                            for a in attribs_str.split(" ")
                            if a.strip()
                        }

                        remove_us = set()
                        for word in attrib_words:
                            if (m := COUNT_IN_ARG_RE.match(word)) is not None:
                                count_in_arg = int(m.group(1))
                                remove_us.add(word)
                            elif (m := CONST_COUNT_RE.match(word)) is not None:
                                const_count = int(m.group(1))
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
                        pieces[0] == "oarg",
                        attribs,
                        count_in_arg,
                        const_count,
                    )
                    arg.arg_type.enrich_base_enum(self.meta)
                    self.func.args.append(arg)
                    continue

                if pieces[0] == "iface":
                    if len(pieces) != 5:
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: iface NAME GROUP VALUE BASETYPE")
                    (name, group_str, value_str, base_type) = pieces[1:5]
                    group, value = int(group_str), int(value_str)
                    self.iface = Interface(name, group, value, MetaType(base_type, 0))
                    self.meta.interfaces.append(self.iface)
                    continue

                if pieces[0] == "meth":
                    if len(pieces) != 4:
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: meth NAME RETTYPE RETSTARS")
                    if self.iface is None:
                        raise ValueError(f"file {txt_path} line {line_number}: \"meth\" entry without a previous \"iface\" entry")
                    (name, ret_type, ret_stars_str) = pieces[1:4]
                    ret_stars = int(ret_stars_str)
                    self.func = Method(name, MetaType(ret_type, ret_stars))
                    self.func.return_type.enrich_base_enum(self.meta)
                    self.iface.methods.append(self.func)
                    continue

                if pieces[0] == "smet":
                    # standard method (returns HRESULT)
                    if len(pieces) != 2:
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: smet NAME")
                    if self.iface is None:
                        raise ValueError(f"file {txt_path} line {line_number}: \"smet\" entry without a previous \"iface\" entry")
                    name = pieces[1]
                    self.func = Method(name, MetaType("HRESULT", 0))
                    self.iface.methods.append(self.func)
                    continue

                if pieces[0] == "inc":
                    if len(pieces) != 2:
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: inc PATH")
                    sub_txt_name = pieces[1]
                    self.collect_path(sub_txt_name)
                    continue

                if pieces[0] == "enum":
                    if len(pieces) != 3:
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: enum NAME BASETYPE")
                    (name, basetype_str) = pieces[1:3]
                    if name in self.meta.name_to_enum:
                        raise ValueError(f"file {txt_path} line {line_number}: duplicate enum named {name!r}")
                    self.enum = Enumeration(name, MetaType(basetype_str, 0))
                    self.meta.name_to_enum[self.enum.name] = self.enum
                    continue

                if pieces[0] == "vnt":
                    if len(pieces) != 3:
                        raise ValueError(f"file {txt_path} line {line_number}: Usage: vnt NAME NUMVALUE")
                    if self.enum is None:
                        raise ValueError(f"file {txt_path} line {line_number}: \"vnt\" entry without a previous \"enum\" entry")
                    (name, num_str) = pieces[1:3]
                    if name in self.enum.name_to_variant:
                        raise ValueError(f"file {txt_path} line {line_number}: duplicate variant {name!r} in enum named {self.enum.name!r}")
                    vnt = EnumVariant(name, int(num_str))
                    self.enum.name_to_variant[vnt.name] = vnt
                    continue

                raise ValueError(f"file {txt_path} line {line_number}: unknown command {pieces[0]!r}")

def run(txt_path: str, il_path: str) -> None:
    collector = CollectorState()
    collector.collect_path(txt_path)

    imported_dlls = sorted(collector.all_dlls)

    env = jinja2.Environment(
        undefined=jinja2.StrictUndefined,
    )
    env.filters["pascal_str_hex_bytes"] = pascal_str_hex_bytes
    env.filters["hex_bytes_le"] = hex_bytes_le
    tpl = env.from_string(TEMPLATE)

    output = tpl.render(
        meta=collector.meta,
        imported_dlls=imported_dlls,
    )

    with open(il_path, "w", encoding="utf-8") as f:
        f.write(output)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        dest="txt_path",
    )
    parser.add_argument(
        dest="il_path",
    )
    args = parser.parse_args()

    run(args.txt_path, args.il_path)


if __name__ == "__main__":
    main()
