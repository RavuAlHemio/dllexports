# metadata.txt -> metadata.il
& python .\metajson2il.py .\metadata.txt .\metadata.il
If ($LASTEXITCODE -ne 0) {
    Exit $LASTEXITCODE
}

# metadata.il -> metadata.winmd
& C:\Windows\Microsoft.NET\Framework64\v4.0.30319\ilasm.exe /dll /output=metadata.winmd metadata.il
