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
and works fine for most modern APIs,
but some programs can't handle them.
PowerShell and `cmd.exe` are examples of such programs.

The `SimplePath` simplifies network share UNC paths
so that such programs can handle.

| | `C:\dir` | `Z:\x` (network) |
| --- | --- | --- |
| [`fs::canonicalize`] | `\\?\C:\dir` | `\\?\UNC\server\share\x` |
| `SimplePath` | `C:\dir` | `\\server\share\x` |
| `SimplePath` with [map to drive] | `C:\dir` | `Z:\x` |

Please see the [library documentation][docs] for more details,
and [releases] for the change history.

## Safety and Equivalence
[safety]: #safety-and-equivalence

Technically speaking,
since the "`\\?\`" prefix ([Win32 File Namespaces])
disables all string parsing and
sends the following string directly to the file system,
simplifying the path is not always guaranteed to be safe or equivalent.

The `SimplePath` simplifies paths
if all the following conditions are met.
* The path is prefixed by "`\\?\UNC\`",
  or "`\\?\C:`" where `C` is an ASCII alphabet letter.
* The path doesn't have any invalid characters or reserved names,
  as defined by the [Naming Conventions].

You can change the following criteria if needed:
* The "long paths" (paths longer than 260 characters) are simplified by default,
  as modern environments can often handle them.
  You can disable simplifying long paths by [`disallow_long`].
* Enable  [`disallow_unknown_unc`] to restrict simplification to verified paths,
  providing an extra layer of safety.

## Examples

When your PC has a network share on the `Z:` drive,
either by the File Explorer or by the command line such as:
```
net use Z: \\server\share
```

Then canonicalizing "`Z:\dir\file`" will be "`\\?\UNC\server\share\dir\file`".
Neither PowerShell nor `cmd.exe` can handle this path.
```rust
let path = r"Z:\dir\file";
let canonicalized = fs::canonicalize(path)?;
println!("{}", canonicalized.display());
```

Using the `SimplePath` prints "`\\server\share\dir\file`" instead.
```rust
let path = r"Z:\dir\file";
let canonicalized = SimplePath::default().canonicalize(path)?;
println!("{}", canonicalized.display());
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
to normalize some other cases by default.
You can skip the [`dunce` crate] simplification
by the [`skip_dunce`] option.
```rust
let simple = SimplePath { skip_dunce: true, ..Default::default() };
```

## Other Platforms

On other platforms than Windows,
the `SimplePath` returns without doing anything.

You can wrap the calls with `#[cfg(windows)]` if you prefer,
though your programs should build and run fine without doing so.

[`disallow_unknown_unc`]: https://docs.rs/simple-path/latest/simple_path/struct.SimplePath.html#structfield.disallow_unknown_unc
[`disallow_long`]: https://docs.rs/simple-path/latest/simple_path/struct.SimplePath.html#structfield.disallow_long
[`dunce` crate]: https://crates.io/crates/dunce
[`fs::canonicalize`]: https://doc.rust-lang.org/std/fs/fn.canonicalize.html
[`map_to_drive`]: https://docs.rs/simple-path/latest/simple_path/struct.SimplePath.html#structfield.map_to_drive
[Naming Conventions]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#naming-conventions
[releases]: https://github.com/kojiishi/simple-path/releases
[`skip_dunce`]: https://docs.rs/simple-path/latest/simple_path/struct.SimplePath.html#structfield.skip_dunce
[Win32 File Namespaces]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
