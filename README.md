# dllexports

Extracts the symbols exported from Windows files with executable code (mostly `.exe` and `.dll`).

Understands NE and PE. (No LE, sorry.)

```bash
dllexports scan DIRECTORY
```

Also allows to poke and prod at specific ancillary information (icon resources, font resources, .dbg files); call `dllexports poke --help` for more information.

A related project is [winapi-history](https://github.com/RavuAlHemio/winapi-history), which can collate and display the collected information.
