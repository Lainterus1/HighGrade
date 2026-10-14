"""Reproducible full pipeline with persistent PNG, SVG and rendered preview."""
import argparse
import base64
import io
import json
from pathlib import Path
import sys

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from PIL import Image
from scripts.vectorize import vectorize, validate_svg, node_tools
from tests.fixtures import logo


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=Path("examples"))
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    source = args.output_dir / "smoke-input.png"
    output = args.output_dir / "smoke-output.svg"
    logo().save(source)
    report = vectorize(source, output, preset="logo", verbose=True)
    svg = output.read_text(encoding="utf-8")
    validate_svg(svg, (192, 128))
    rendered = node_tools({"svg": svg, "optimize": False, "preview": True})
    png = base64.b64decode(rendered["png"])
    image = Image.open(io.BytesIO(png)).convert("RGBA")
    assert image.getpixel((64, 64))[3] == 0, "transparent hole lost"
    assert abs(image.getpixel((145, 64))[3] - 128) <= 2, "partial opacity lost"
    assert image.getpixel((32, 64))[3] == 255, "opaque shape lost"
    (args.output_dir / "smoke-rendered.png").write_bytes(png)
    report["pipeline"] = ["raster", "VTracer", "SVGO", "XML + resvg validation", "output.svg"]
    (args.output_dir / "smoke-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print("SMOKE PASS: " + " -> ".join(report["pipeline"]))
    print(output.resolve())


if __name__ == "__main__":
    main()
