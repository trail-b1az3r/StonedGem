# GPU code

A `.zsg` file holds code meant for a GPU: **kernels**, which do general-purpose work on lists of numbers, and **fragment** shaders, which compute a colour for every pixel of an area. Each file is type-checked and compiled to WGSL, the shading language of WebGPU, and the result is validated with naga (the shader compiler wgpu uses). The same code also runs on the CPU, which is how programs use it today and what the tests compare against.

```zsg include=examples/zsharp/rings.zsg
```

## Using it from a program

Importing a `.zsg` file makes each kernel and fragment a global. A kernel's `run(buffers, uniforms)` runs it and returns the buffers; a canvas's `shade(fragment, x, y, w, h, uniforms)` fills an area with a fragment.

```zsp run include=examples/zsharp/rings.zsp
```

## Kernels

```zsg
kernel scale(buf data: [float], uniform k: float) {
    data[gid] = data[gid] * k
}
```

- Parameters are either `buf name: [float]` / `[int]` (a buffer) or `uniform name: type` (one value for the whole run). A kernel needs at least one buffer.
- It runs once per element of its **first** buffer, with that element's index in `gid`. Every run is independent: write only to your own element of an output buffer.
- In WGSL each kernel is a `@compute @workgroup_size(64)` entry point, and each entry point in a module gets its own bind group.

## Fragments

```zsg
fragment glow(uv: vec2, uniform time: float) -> color {
    let d = length(uv - vec2(0.5, 0.5))
    return vec4(0.2, 0.5, 1.0, 1.0) * (1.0 - d) * (0.8 + 0.2 * sin(time))
}
```

- The first parameter is always `uv: vec2`, from `(0, 0)` at the top-left of the area to `(1, 1)` at the bottom-right. `resolution` (a `vec2`, in pixels) is always available.
- Other parameters are uniforms.
- It returns `color` (the same as `vec4`): red, green, blue and alpha from 0 to 1.

## The language on the GPU

`.zsg` is the same syntax, restricted to what GPUs can do, and every value has a type known when it compiles.

| Types | `float`, `int`, `bool`, `vec2`, `vec3`, `vec4` (or `color`); buffers `[float]`, `[int]` |
|---|---|
| Vectors | `vec3(0.1, 0.2, 0.3)`, `vec4(c, 1.0)` from a `vec3` and a number; fields `.x .y .z .w` and `.r .g .b .a`, and swizzles like `.xy` or `.rgb` |
| Statements | `let` / `var` (a type or a value is required), assignment and `+=` etc., `if` / `else`, `while`, `for i in a..b`, `return`, `break`, `continue` |
| Declarations | `const` (numbers), `func` helpers with typed parameters and a return type |
| Built-ins | `sin cos tan sqrt abs floor ceil fract exp log sign round min max clamp mix step smoothstep pow atan2 length distance dot normalize vec2 vec3 vec4 float int len` |
| Not available | strings, lists, maps, structs, lambdas, `match`, `try`/`throw`, recursion |

Numbers and vectors mix the way shaders expect: `vec3 * float` scales, `vec3 + vec3` adds per channel. `int` values are converted to `float` where a float is needed.

Mistakes are reported with the line and what's wrong:

```zsg error
kernel blur(buf xs: [float]) {
    let v = vec2(1.0, 2.0)
    xs[gid] = v.z
}
```

## Seeing the WGSL

`zspp wgsl file.zsg` prints the WGSL a module compiles to, and a program can ask for it with `kernel.wgsl()`. A host that has a GPU can hand this WGSL to wgpu (or any WebGPU implementation): buffers and uniforms are laid out as the bind groups in the output describe.
