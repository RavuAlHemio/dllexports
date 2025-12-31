#!/bin/sh
# metadata.txt -> metadata.il
python ./metatext2il.py ./metadata.txt ./metadata.il
e="$?"
if [ "$e" -ne 0 ]
then
	exit "$e"
fi

# metadata.il -> IgorPavlov.SevenZip.winmd
if [ ! -x "./ilasm" ]
then
	cat >&2 <<_END
You are missing ilasm in the current directory.

You can obtain it from the NuGet package "runtime.linux-x64.Microsoft.NETCore.ILAsm"
(or a different runtime than linux-x64, depending on your computer and operating system);
simply download the .nupkg file and extract it using a ZIP-capable program.
_END
	exit 1
fi

if [ ! -f "./Windows.Win32.winmd" ]
then
	cat >&2 <<_END
You are missing Windows.Win32.winmd in the current directory.

You can obtain it from the NuGet package "Microsoft.Windows.SDK.Win32Metadata";
simply download the .nupkg file and extract it using a ZIP-capable program.
_END
	exit 1
fi

./ilasm -dll -output=IgorPavlov.SevenZip.winmd metadata.il
