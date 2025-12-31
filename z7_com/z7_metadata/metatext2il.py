#!/usr/bin/env python3

import argparse
import jinja2
import metatext


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
        {% if
            arg.attributes
            or arg.count_in_arg is not none
            or arg.const_count is not none
            or arg.arg_type.base_enum is not none
        -%}
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
                        {{ arg.count_in_arg|hex_bytes_le(2) }} // {{ arg.count_in_arg }}
                    )
                {% elif arg.const_count is not none -%}
                    .custom instance void [Windows.Win32.winmd]Windows.Win32.Foundation.Metadata.NativeArrayInfoAttribute::.ctor() = (
                        01 00 01 00
                        53 08
                        0A 43 6F 75 6E 74 43 6F 6E 73 74 // length prefix plus "CountConst"
                        {{ arg.const_count|hex_bytes_le(4) }} // {{ arg.const_count }}
                    )
                {% endif %}
                {% if arg.arg_type.base_enum is not none -%}
                    .custom instance void [Windows.Win32.winmd]Windows.Win32.Foundation.Metadata.AssociatedEnumAttribute::.ctor(string) = (
                        01 00
                        {{ arg.arg_type.base_enum|ser_string_hex_bytes }} // {{ arg.arg_type.base_enum }}
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
.class public auto autochar sealed beforefieldinit {{ meta.name }}.{{ fptr.name }}
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

{% if meta.funcs or meta.guid_consts -%}
.class public auto autochar abstract sealed beforefieldinit {{ meta.name }}.Apis
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
    {% for guid_const in meta.guid_consts -%}
    .field public static valuetype [netstandard]System.Guid '{{ guid_const.name }}'
    .custom instance void [Windows.Win32.winmd]Windows.Win32.Foundation.Metadata.GuidAttribute::.ctor(
        uint32, uint16, uint16, uint8, uint8, uint8, uint8, uint8, uint8, uint8, uint8) = (
            01 00
            {{ guid_const.guid_bytes_le|hex_bytes }} // {{ guid_const.guid_str }}
            00 00 )
    {% endfor %}
}
{% endif %}{# meta.funcs #}

{% for iface in meta.interfaces -%}
.class interface public abstract auto ansi {{ meta.name }}.{{ iface.name }}
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
  {% if enum.is_flags -%}
  .custom instance void [netstandard]System.FlagsAttribute::.ctor() = ( 01 00 00 00 )
  {% endif %}
  .field public specialname rtspecialname {{ enum.base_type.il_type(meta) }} value__
  {% for variant in enum.name_to_variant.values() %}
  .field public static literal valuetype {{ meta.name }}.{{ enum.name }} {{ variant.name }} = {{ enum.base_type.il_type(meta) }}({{ variant.value }})
  {% endfor %}
}
{% endfor %}{# meta.name_to_enum #}

{% for struct in meta.structs %}
.class public sequential ansi sealed beforefieldinit {{ meta.name }}.{{ struct.name }}
       extends [netstandard]System.ValueType
{
  {% for field in struct.fields %}
  .field public {{ field.field_type.il_type(meta) }} '{{ field.name }}'
  {% endfor %}
}
{% endfor %}{# meta.structs #}
"""


def hex_bytes(bs: bytes) -> str:
    return " ".join(f"{b:02X}" for b in bs)


def ser_string_hex_bytes(text: str) -> str:
    encoded = text.encode("utf-8")
    length = len(encoded)

    if length <= 0x7F:
        # 0xxx_xxxx
        length_str = f"{length:02X}"
    elif length <= 0x3FFF:
        # 10xx_xxxx xxxx_xxxx
        top_byte = (((length >> 8) & 0x3F) | 0b1000_0000)
        bottom_byte = ((length >> 0) & 0xFF)
        length_str = f"{top_byte:02X} {bottom_byte:02X}"
    elif length <= 0x1FFFFFFF:
        # 110x_xxxx xxxx_xxxx xxxx_xxxx xxxx_xxxx
        msb = (((length >> 24) & 0x1F) | 0b1100_0000)
        mb1sb = ((length >> 16) & 0xFF)
        mb2sb = ((length >>  8) & 0xFF)
        lsb = ((length >>  0) & 0xFF)
        length_str = f"{msb:02X} {mb1sb:02X} {mb2sb:02X} {lsb:02X}"
    else:
        raise ValueError("text too long for SerString")

    text_hex = hex_bytes(encoded)
    return f"{length_str} {text_hex}"


def hex_bytes_le(number: int, byte_count: int) -> str:
    return " ".join(f"{b:02X}" for b in number.to_bytes(byte_count, "little"))


def run(txt_path: str, il_path: str) -> None:
    collector = metatext.CollectorState()
    collector.collect_path(txt_path)

    imported_dlls = sorted(collector.all_dlls)

    env = jinja2.Environment(
        undefined=jinja2.StrictUndefined,
    )
    env.filters["hex_bytes"] = hex_bytes
    env.filters["ser_string_hex_bytes"] = ser_string_hex_bytes
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
