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

Please see the [documentation][docs] for more details,
and [releases] for the change history.

## Safety

Technically speaking,
the "`\\?\`" prefix ([Win32 File Namespaces]) is
to disable all string parsing and
to send the string that follows it straight to the file system.

For this reason,
simplifying them is not guaranteed to be safe.
The `SimplePath` simplifies them
if all the following conditions are true.
* It is prefixed by "`\\?\UNC\`" (not only "`\\?\`").
  - Note: other prefixes such as "`\\?\C:`" are simplified by [`dunce`],
    which is included by default.
* It doesn't have any invalid characters defined in the [Naming Conventions].
* The UNC is "connected" on the PC.
  That is,
  the UNC is shown in the File Explorer,
  or in the list of connections when you run `net use` from the command line.
  - Note: this condition can be relaxed by [`allow_unknown_unc`].

The "long paths" (paths longer than 260 characters) are simplified by default.
You can disable this behavior by [`disallow_long`].

## Examples

When your PC has a network share on the `Z:` drive,
either by the File Explorer or by the following command line:
```
net use Z: \\server\share
```

The following code prints "`\\?\UNC\server\share\dir\file`".
Neither PowerShell nor `cmd.exe` can handle this path.
```rust
let path = r"Z:\dir\file";
let canonicalized = fs::canonicalize(path)?;
println!("{}", canonicalized.display());
```

Using the `SimplePath` prints "`\\server\share\dir\file`" instead.
```rust
let path = r"Z:\dir\file";
let simplified = SimplePath::default().canonicalize(path)?;
println!("{}", simplified.display());
```

This path works fine for PowerShell and `cmd.exe`.

## Map to Drive
[map to drive]: #map-to-drive

If you prefer network drive names instead of UNC (`\\server\share`"),
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
[`dunce`]: #dunce

The `SimplePath` calls the [`dunce` crate]
to normalize some other cases, such as:
`\\?\C:\foo` to `C:\foo`.
You can skip the [`dunce` crate] simplification if you prefer.
```rust
let simple = SimplePath { skip_dunce: true, ..Default::default() };
```

## Other Platforms

On other platforms than Windows,
the `SimplePath` returns without doing anything.

You can wrap the calls with `#[cfg(windows)]` if you prefer,
though your programs should build and run fine without doing so.

[`allow_unknown_unc`]: https://docs.rs/simple-path/latest/simple_path/struct.SimplePath.html#structfield.allow_unknown_unc
[`disallow_long`]: https://docs.rs/simple-path/latest/simple_path/struct.SimplePath.html#structfield.disallow_long
[`dunce` crate]: https://crates.io/crates/dunce
[`fs::canonicalize`]: https://doc.rust-lang.org/std/fs/fn.canonicalize.html
[`map_to_drive`]: https://docs.rs/simple-path/latest/simple_path/struct.SimplePath.html#structfield.map_to_drive
[Naming Conventions]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#naming-conventions
[releases]: https://github.com/kojiishi/simple-path/releases
[Win32 File Namespaces]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
