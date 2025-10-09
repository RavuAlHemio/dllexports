# metadata.txt -> metadata.il
& python .\metajson2il.py .\metadata.txt .\metadata.il
If ($LASTEXITCODE -ne 0) {
    Exit $LASTEXITCODE
}

# metadata.il -> metadata.winmd

If (-not (Test-Path -Path ".\ilasm.exe")) {
    Write-Error -Message "

You are missing ilasm.exe in the current directory.

You can obtain it from the NuGet package `"runtime.win-x64.Microsoft.NETCore.ILAsm`"
(or a different runtime than win-x64, depending on your computer and operating system);
simply download the .nuget file and extract it using a ZIP-capable program."
    Exit 1
}

If (-not (Test-Path -Path ".\Windows.Win32.winmd")) {
    Write-Error -Message "

You are missing Windows.Win32.winmd in the current directory.

You can obtain it from the NuGet package `"Microsoft.Windows.SDK.Win32Metadata`"
simply download the .nuget file and extract it using a ZIP-capable program."
    Exit 1
}

& .\ilasm.exe /dll /output=metadata.winmd metadata.il
