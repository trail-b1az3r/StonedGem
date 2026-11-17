# Z# and Z#++

**Z#** here is [Sam-astro's Z-Sharp](https://github.com/sam-astro/Z-Sharp)
(MIT): a small interpreted language (about 4,500 lines of C++) with
`func`, C-like syntax and `.zs` files.

## Why it can't be used as it is

- It builds only as a standalone program (no library API).
- It needs SDL2 and Boost, for its graphics and its `any` type.
- Scripts can run shell commands (`ZS.System.Command`) and read files,
  which a browser must never allow page- or extension-supplied code to do.
- It targets C++17; StonedGem's C++ is C++23.

## Z#++: the plan (phase 7)

Z#++ is a fork, vendored under `third_party/zsharp/` with its licence:

1. **Library build**: the interpreter as a C++23 library with a small C
   ABI (`zspp_new`, `zspp_eval`, `zspp_call`, `zspp_free`) and a Rust
   wrapper crate, `stonedgem-zsharp`. SDL2 graphics become optional and
   off; `boost::any` becomes `std::any`.
2. **Sandbox**: no shell, no files, no network. Scripts get only the host
   functions StonedGem registers, each behind a permission.
3. **Syntax extensions** (what makes it Z#++): typed `let` declarations,
   `on <event>` handlers, string interpolation, and modules.
4. **Use in StonedGem**: user automation scripts — "on page load at
   example.com, do …", keyboard macros, tab rules — with a browser API
   (`Tabs`, `Page`, `Notify`) exposed to the interpreter.

Nothing of Z#++ exists yet; this file is the plan.
