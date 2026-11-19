# Writing these docs

This site is built from the Markdown files in `docs/zsharp/` by a small Rust program, `crates/stonedgem-zsharp-docs`. It has no JavaScript framework and no theme; the look is set out in [`docs/zsharp/DESIGN.md`](https://github.com/trail-b1az3r/StonedGem/blob/main/docs/zsharp/DESIGN.md).

## Building

```sh
cargo run -p stonedgem-zsharp-docs              # writes target/zsharp-docs/
cargo run -p stonedgem-zsharp-docs -- out/dir   # or somewhere else
```

Open `target/zsharp-docs/index.html` in a browser; the pages work from disk. To publish, copy that folder to any static host (GitHub Pages included: the build writes the `.nojekyll` file Pages needs).

## Pages

`docs/zsharp/nav.txt` lists the pages in order, under `# Section` lines. Each page starts with a `# Title`. Level-2 headings (`##`) become the “On this page” list. Links between pages use the `.md` file name; the build turns them into `.html`.

## Examples are run

Every code block in a Z#++ language is checked against the real implementation when the site is built, and a failing example stops the build. What happens depends on the block's info string:

| Info string | Checked by | Shown |
|---|---|---|
| ` ```zsp ` | parsing it as a program | the code |
| ` ```zsp run ` | loading it and running `Main()` | the code, its **Output**, and the **Canvas** if it keeps one in `canvas` or `screen` |
| ` ```zsp error ` | running it; it must fail | the code and the **Error** |
| ` ```zsx ` / ` ```zsx error ` | parsing it as a header | the code (and the error) |
| ` ```zsg ` / ` ```zsg error ` | type-checking and compiling to WGSL, validated by naga | the code (and the error) |
| ` ```zsp run include=examples/x.zsp ` | the same, on a file from the repository (its imports are read next to it) | the file, linked to its source |
| ` ```rust `, ` ```sh `, ` ```text ` | nothing | the code |

The test suite builds the whole site (`cargo test -p stonedgem-zsharp-docs`), so CI catches an example that a language change broke. It also checks that every built-in global appears in the [Reference](reference.md).

## Style

- Say what something does, then show it. Prefer one runnable example to three fragments.
- Use the reader's words: “a list of numbers”, not “a `Value::List` of `Value::Float`”, except on the Rust pages.
- Callouts are blockquotes that start with **Note** or **Careful**. Use them rarely.
- No marketing: no “blazing”, “seamless” or “powerful”. If something is fast, give the number.
