# SimpleUnc

On Windows,
[`fs::canonicalize`] returns a path in the [Win32 File Namespaces],
which is prefixed by "`\\?\`",
such as:
```
\\?\UNC\server\share\dir
```
The "`\\?\`" prefix has advantages such as long paths,
and is fine for most modern APIs,
but some programs can't handle them.
PowerShell and `cmd.exe` are examples of such programs.

The `SimpleUnc` simplifies network share UNC paths
so that these programs can handle,
if they are currently mapped to drives.

Let's say your PC has a network share on the `Z:` drive:
```
net use Z: \\server\share
```
If you run the following code on this PC:
```rust
let path = r"Z:\dir\file";
let canonicalized = fs::canonicalize(path)?;
println!("{}", canonicalized.display());
```
prints:
```
\\?\UNC\server\share\dir\file
```
Neither PowerShell nor `cmd.exe` can handle this path.

With the `SimpleUnc`:
```rust
let path = r"Z:\dir\file";
let simplified = SimpleUnc::default().canonicalize(path)?;
println!("{}", simplified.display());
```
prints:
```
Z:\dir\file
```
This path works fine for PowerShell and `cmd.exe`.

[`dunce`]: https://crates.io/crates/dunce
[`fs::canonicalize`]: https://doc.rust-lang.org/std/fs/fn.canonicalize.html
[Win32 File Namespaces]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
