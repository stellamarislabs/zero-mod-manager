# Portable data modes

The portable ZIP does not need an installer. By default, the executable uses
the normal per-user application data folders so replacing or moving the EXE
does not lose profiles, settings, or the managed library.

For a completely self-contained copy, create an empty file named
`portable-data.flag` beside `Zero Mod Manager.exe` before the first launch.
The application will then use a `data` folder beside the executable.

The containing folder must be writable. Do not enable self-contained mode in
`Program Files`, a read-only drive, or directly inside the ZIP. Extract the ZIP
to a normal folder first.
