---
name: vectorize
description: Convert PNG/JPG/WebP to SVG; vectorize raster logos, icons, illustrations and line art with local VTracer and SVGO. Use for raster-to-vector tracing, including «векторизуй» and «переведи PNG в SVG».
---

# Raster to SVG

Use the bundled deterministic tool for raster tracing. VTracer builds the paths;
SVGO optimizes them; XML/path checks and a local resvg render validate the output.
No MCP server or network service is involved in conversion.

## Invocation

Resolve the plugin root from this file: two directories above the `vectorize`
skill directory. Use absolute, properly quoted input/output paths. Prefer the
plugin's `.venv/Scripts/python.exe` on Windows or `.venv/bin/python` on Unix.
The script is [scripts/vectorize.py](../../scripts/vectorize.py).

```text
<plugin-python> <plugin-root>/scripts/vectorize.py <input> -o <output.svg> --preset <preset> --verbose
```

With an activated environment and the editable CLI installed, use:

```bash
vectorize logo.png -o logo.svg --preset logo
vectorize --help
```

Honor the user's explicit preset. Otherwise choose from the image type described
in the request or visible in the image:

| Type | Preset |
|---|---|
| Logo / brand mark, few colors, distinct outer contours | `logo` |
| Small icon / pictogram, simple geometry, transparent background | `icon` |
| Drawing / illustration / anime character with many colors or small details | `illustration` |
| Monochrome sketch / ink / contour drawing | `line-art` |

When intent does not establish a type, omit `--preset` to use the tool's
deterministic image heuristic. Report the selected preset; allow the user to
override it. Do not treat automatic classification as certain.

`illustration` defaults to the visually selected B configuration: VTracer
1.0.0a4, Lanczos 2x before a 128-color palette, stacked tracing and SVGO.
Use these defaults for detailed color artwork; no candidate search is needed.
The tool reduces noise filtering on small inputs to preserve detached details.
Use `--scale 1` for an input exceeding the 4-megapixel limit at scale 2;
report that choice. Scaling never changes the final SVG dimensions.

Explicit `--colors 2..256` limits the color palette via deterministic preprocessing
and snaps any extra averaged color fills to that palette;
alpha-mask grays are separate. `line-art` only accepts 2 colors.
`--simplify 0..10` increases VTracer curve simplification as the value grows.
`--filter-speckle 0` preserves tiny components. `--no-optimize` explicitly skips
SVGO but keeps validation. See [README](../../README.md) for installation and
limits; do not assume a system Python or globally installed SVGO is sufficient.

## Non-negotiable tracing behavior

- Do not manually reconstruct complex raster artwork by writing `path d` coordinates.
- Do not replace a missing dependency or failed vectorizer with manual SVG tracing,
  an LLM-generated approximation, or an embedded PNG inside an SVG.
- On failure, show the actual error and the suggested installation/fix command.
  Install dependencies only within the current authorization. Fix or retry the
  deterministic pipeline; if it remains blocked, report that no SVG was produced.
- Do not silently add `--no-optimize` when SVGO is missing. This changes the
  requested pipeline and must be explicit.
- After successful conversion, small edits to colors, viewBox, dimensions,
  IDs/classes, simple geometry, and obvious noise are allowed. Keep tracing
  deterministic; do not replace complex paths with hand-written coordinates.
- A genuinely simple new SVG primitive such as one circle, arrow, or rectangle
  may be authored manually without running the vectorizer.

Inspect the result when visual fidelity matters. Pay attention to holes,
transparency, text, small detached marks, and thin lines. If a preset removes a
meaningful detail, lower `--filter-speckle` / `--simplify` or use `illustration`
and rerun the tool. Do not label inferred geometry as an exact reconstruction.
Return the output file and relevant limitations. Creating this plugin does not
by itself install it into the user's Codex plugin catalog.
