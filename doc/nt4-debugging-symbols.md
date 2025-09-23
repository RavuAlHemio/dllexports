# Obtaining Windows NT 4-style debugging symbols

In the Windows NT 4 era, debugging symbols were mostly distributed as `.dbg` files, not as `.pdb` files. (`.pdb` files did exist, but their role was limited to the build process, i.e. passing debugging information between the compiler and the linker.)

With Visual C++ 4.0 (`cl.exe` and `link.exe`) and the Windows NT 4 SDK (`rebase.exe`), the following combination of steps can be taken to obtain `.dbg` files:

1. Compile your C/C++ code with `cl.exe /c` using the `/Z7` option, which embeds debug information directly in the `.obj` file.

2. Link your executable or DLL with `link.exe` using the options `/DEBUG /DEBUGTYPE:BOTH /PDB:NONE`. The options work as follows: `/DEBUG` ensures that debug information is output at all, `/DEBUGTYPE:BOTH` ensures that both COFF and CodeView information is output, and `/PDB:NONE` ensures that the debug symbols are embedded into the executable or DLL instead of generating an external `.pdb` file.

3. Separate out the debug symbols by using `rebase.exe -b 0x400000 -x dbgsym EXEORDLL`. Replace `0x400000` with the base address of the image (if unsure, run `dumpbin.exe /headers EXEORDLL` and check `size of image` in the `OPTIONAL HEADER VALUES` section), since we do not actually want to rebase the image.

A new subdirectory named `dbgsym` will be created with a sub-sub-directory `dll` or `exe` which contains the `.dbg` file for your executable or DLL. Simultaneously, the debugging information in the executable or DLL will be replaced with a reference to the `.dbg` file.

And that's it!

(The symbol-splitting functionality was removed from `rebase.exe` sometime between Windows 2000 and Windows Server 2003; `imagehlp.dll` however contains the underlying `SplitSymbols` function since NT 3.5, as well as Windows 95 OSR2 for the Win9x branch, until today.)
