[![CI-badge]][CI]
[![crate-badge]][crate]
[![docs-badge]][docs]

[CI-badge]: https://github.com/kojiishi/simple-path/actions/workflows/rust-ci.yml/badge.svg
[CI]: https://github.com/kojiishi/simple-path/actions/workflows/rust-ci.yml
[crate-badge]: https://img.shields.io/crates/v/simple-path.svg
[crate]: https://crates.io/crates/simple-path
[docs-badge]: https://docs.rs/simple-path/badge.svg
[docs]: https://docs.rs/simple-path/

# SimplePath

On Windows,
[`fs::canonicalize`] returns a path prefixed by "`\\?\`".
such as:
```
\\?\UNC\server\share\dir
```
The "`\\?\`" prefix is called the [Win32 File Namespaces].
It has advantages such as long paths,
and is fine for most modern APIs,
but some programs can't handle them.
PowerShell and `cmd.exe` are examples of such programs.

The `SimplePath` simplifies network share UNC paths
so that these programs can handle.

| | `C:\dir` | `Z:\x` (network) |
| --- | --- | --- |
| [`fs::canonicalize`] | `\\?\C:\dir` | `\\?\UNC\server\share\x` |
| `SimplePath` | `C:\dir` | `\\server\share\x` |
| `SimplePath` with [map to drive] | `C:\dir` | `Z:\x` |

Since the simplification may not always be safe,
it does so if they are currently mapped to drives.

Please see the [documentation][docs] for more details.

## Examples

When your PC has a network share on the `Z:` drive:
```
net use Z: \\server\share
```
If you run the following code on this PC:
```rust
let path = r"Z:\dir\file";
let canonicalized = fs::canonicalize(path)?;
println!("{}", canonicalized.display());
```
prints "`\\?\UNC\server\share\dir\file`".
Neither PowerShell nor `cmd.exe` can handle this path.

The `SimplePath` prints "`\\server\share\dir\file`" instead.
```rust
let path = r"Z:\dir\file";
let simplified = SimplePath::default().canonicalize(path)?;
println!("{}", simplified.display());
```

This path works fine for PowerShell and `cmd.exe`.

## Map to Drive
[map to drive]: #map-to-drive

If you prefer network drive names instead of UNC,
enable the [`map_to_drive`] option.

The following code prints "`Z:\dir\file`"
instead of "`\\server\share\dir\file`".
```rust
let path = r"Z:\dir\file";
let simple = SimplePath {
    map_to_drive: true,
    ..Default::default()
};
let simplified = simple.canonicalize(path)?;
println!("{}", simplified.display());
```

## Dunce

The `SimplePath` calls the [`dunce`] crate
to normalize some other cases, such as:
`\\?\C:\foo` to `C:\foo`.
You can skip the [`dunce`] simplification if you prefer:
```rust
let simple = SimplePath { skip_dunce: true, ..Default::default() };
```

## Other Platforms

On other platforms than Windows,
the `SimplePath` returns without doing anything.

You can wrap the calls with `#[cfg(windows)]` if you prefer,
though your programs should build and run fine without doing so.

[`dunce`]: https://crates.io/crates/dunce
[`fs::canonicalize`]: https://doc.rust-lang.org/std/fs/fn.canonicalize.html
[`map_to_drive`]: https://docs.rs/simple-path/latest/simple_path/struct.SimplePath.html#structfield.map_to_drive
[Win32 File Namespaces]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
