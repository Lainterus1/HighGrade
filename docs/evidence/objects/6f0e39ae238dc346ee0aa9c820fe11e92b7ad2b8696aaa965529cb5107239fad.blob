#!/usr/bin/env python3
"""VTracer -> explicit SVGO pass -> XML + resvg validation -> atomic SVG output."""
from __future__ import annotations

import argparse
import importlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import warnings
import xml.etree.ElementTree as ET

SVG = "http://www.w3.org/2000/svg"
ET.register_namespace("", SVG)
ROOT = Path(__file__).resolve().parent.parent
BRIDGE = Path(__file__).resolve().with_name("svg_tools.mjs")
MAX_PIXELS = 16_000_000
PRESETS = {
    "logo": {"colors": 8, "simplify": 7, "speckle": 8, "corner": 60},
    "icon": {"colors": 6, "simplify": 8, "speckle": 4, "corner": 45},
    "illustration": {"colors": 128, "simplify": 2, "speckle": None, "corner": 60},
    "line-art": {"colors": 2, "simplify": 1, "speckle": 2, "corner": 60},
}


class VectorizeError(Exception):
    """Actionable error safe to show without a traceback."""


def python_dependency(module, label, package):
    try:
        return importlib.import_module(module)
    except (ImportError, OSError) as error:
        raise VectorizeError(
            f'{label} не установлен или не загружается. Выполните '
            f'"{sys.executable}" -m pip install {package}. {error}'
        ) from error


def node_tools(request):
    node = shutil.which("node")
    if not node:
        raise VectorizeError("Node.js не установлен. Установите Node.js 20+ с https://nodejs.org/ и выполните npm ci --ignore-scripts в корне плагина.")
    try:
        result = subprocess.run(
            [node, str(BRIDGE)], input=json.dumps(request), capture_output=True,
            text=True, encoding="utf-8", timeout=120, check=False,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise VectorizeError(f"Не удалось запустить SVG processing/validation: {error}") from error
    if result.returncode:
        raise VectorizeError(result.stderr.strip() or "Ошибка SVG processing/validation.")
    try:
        return json.loads(result.stdout)
    except ValueError as error:
        raise VectorizeError("Невалидный ответ SVG processing/validation.") from error


def load_image(path):
    if not path.is_file():
        raise VectorizeError(f"Входной файл отсутствует: {path}")
    formats = {".png": "PNG", ".jpg": "JPEG", ".jpeg": "JPEG", ".webp": "WEBP"}
    if path.suffix.lower() not in formats:
        raise VectorizeError("Неподдерживаемый формат. Нужен PNG, JPG/JPEG или WebP.")
    Image = python_dependency("PIL.Image", "Pillow", "Pillow==12.3.0")
    ImageOps = python_dependency("PIL.ImageOps", "Pillow", "Pillow==12.3.0")
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("error", Image.DecompressionBombWarning)
            with Image.open(path) as source:
                if source.format != formats[path.suffix.lower()]:
                    raise VectorizeError("Неподдерживаемый формат: содержимое файла не соответствует расширению.")
                if getattr(source, "n_frames", 1) != 1:
                    raise VectorizeError("Анимация не поддерживается. Сначала сохраните нужный кадр как PNG.")
                if source.width * source.height > MAX_PIXELS:
                    raise VectorizeError("Изображение больше 16 мегапикселей. Сначала уменьшите его размер.")
                return ImageOps.exif_transpose(source).convert("RGBA")
    except VectorizeError:
        raise
    except (OSError, ValueError, Image.DecompressionBombError, Image.DecompressionBombWarning) as error:
        raise VectorizeError(f"Не удалось прочитать изображение: {error}") from error


def choose_preset(image):
    """Bounded, deterministic image heuristic; explicit user intent always wins."""
    sample = image.copy()
    sample.thumbnail((128, 128))
    pixels = [p for p in sample.get_flattened_data() if p[3] > 32]
    if not pixels:
        return "icon"
    gray = sum(max(p[:3]) - min(p[:3]) <= 12 for p in pixels) / len(pixels)
    extremes = sum(max(p[:3]) < 80 or min(p[:3]) > 190 for p in pixels) / len(pixels)
    if gray > 0.98 and extremes > 0.9:
        return "line-art"
    colors = len({tuple(v // 16 for v in p[:3]) for p in pixels})
    if colors <= 16:
        return "icon" if max(image.size) <= 256 else "logo"
    return "illustration"


def validate_path(data):
    """Check command arities and finite coordinates; XML alone cannot do this."""
    number = r"[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?"
    tokens = re.findall(rf"[A-Za-z]|{number}", data)
    residue = re.sub(rf"[MmLlHhVvCcSsQqTtAaZz]|{number}|[\s,]", "", data)
    if residue or not tokens or tokens[0] not in ("M", "m"):
        raise VectorizeError("Невалидный SVG: некорректный path d.")
    arities = {"M": 2, "L": 2, "H": 1, "V": 1, "C": 6, "S": 4, "Q": 4, "T": 2, "A": 7, "Z": 0}
    index = 0
    while index < len(tokens):
        command = tokens[index].upper()
        if command not in arities:
            raise VectorizeError("Невалидный SVG: неизвестная команда path.")
        index += 1
        values = []
        while index < len(tokens) and not tokens[index].isalpha():
            values.append(float(tokens[index]))
            index += 1
        arity = arities[command]
        if (arity == 0 and values) or (arity and (not values or len(values) % arity)):
            raise VectorizeError("Невалидный SVG: неверное число координат path.")
        if not all(math.isfinite(value) for value in values):
            raise VectorizeError("Невалидный SVG: бесконечная координата path.")
        if command == "A":
            for start in range(0, len(values), 7):
                if values[start] < 0 or values[start + 1] < 0 or any(values[start + flag] not in (0, 1) for flag in (3, 4)):
                    raise VectorizeError("Невалидный SVG: параметры дуги path.")


def validate_svg(svg, size):
    """Validate our generated SVG subset; rendering is checked separately by resvg."""
    if re.search(r"<!DOCTYPE|<!ENTITY", svg, re.I):
        raise VectorizeError("Невалидный SVG: DTD/entities не разрешены.")
    try:
        root = ET.fromstring(svg)
    except ET.ParseError as error:
        raise VectorizeError(f"Невалидный SVG XML: {error}") from error
    if root.tag != f"{{{SVG}}}svg":
        raise VectorizeError("Невалидный SVG: отсутствует корневой svg с namespace.")
    if root.get("viewBox") != f"0 0 {size[0]} {size[1]}":
        raise VectorizeError("Невалидный SVG: неверный viewBox.")
    if root.get("width") != str(size[0]) or root.get("height") != str(size[1]):
        raise VectorizeError("Невалидный SVG: неверные размеры.")
    ids, references = set(), set()
    for element in root.iter():
        if element.tag not in {f"{{{SVG}}}{name}" for name in ("svg", "g", "path", "defs", "mask", "rect")}:
            raise VectorizeError(f"Невалидный SVG: неожиданный элемент {element.tag}.")
        if element.tag == f"{{{SVG}}}path":
            validate_path(element.get("d", ""))
        identity = element.get("id")
        if identity:
            if identity in ids:
                raise VectorizeError("Невалидный SVG: повторяющийся ID.")
            ids.add(identity)
        for key, value in element.attrib.items():
            if key.lower().startswith("on") or "href" in key.lower():
                raise VectorizeError("Невалидный SVG: активное или внешнее содержимое.")
            if "url(" in value:
                if not re.fullmatch(r"url\(#[A-Za-z0-9_-]+\)", value):
                    raise VectorizeError("Невалидный SVG: внешняя ссылка.")
                references.add(value[5:-1])
    if not references.issubset(ids):
        raise VectorizeError("Невалидный SVG: не найдена маска по ID.")
    return root


def trace(image, engine, settings):
    try:
        rgba = image.convert("RGBA")
        svg = engine.Config(**settings).convert_pixels(rgba.tobytes(), *rgba.size)
        root = ET.fromstring(svg)
        if root.tag != f"{{{SVG}}}svg":
            raise ValueError("VTracer returned a non-SVG document")
        # VTracer can emit empty paths for tiny JPEG clusters. They have no paint;
        # deleting these is not a geometry fallback and avoids invalid empty paths.
        return [child for child in root
                if child.tag != f"{{{SVG}}}metadata"
                and not (child.tag == f"{{{SVG}}}path" and not child.get("d", "").strip())]
    except BaseException as error:
        # PyO3 PanicException inherits BaseException; never swallow user cancellation.
        if isinstance(error, (KeyboardInterrupt, SystemExit, GeneratorExit)):
            raise
        raise VectorizeError(f"Ошибка векторизации VTracer: {error}") from error


def build_svg(image, preset, engine, colors=None, simplify=None, filter_speckle=None, scale=None):
    from PIL import Image, ImageOps
    config = PRESETS[preset]
    limit_output_colors = colors is not None
    colors = config["colors"] if colors is None else colors
    simplify = config["simplify"] if simplify is None else simplify
    speckle = config["speckle"] if filter_speckle is None else filter_speckle
    if speckle is None:
        # Noise filtering must track source resolution: preserve tiny illustrations,
        # reach the selected B threshold (4 source px) at a 1280 px longest side.
        speckle = min(4, max(image.size) // 320)
    scale = (2 if preset == "illustration" else 1) if scale is None else scale
    if image.width * image.height * scale**2 > MAX_PIXELS:
        raise VectorizeError("Трассировка с увеличением превышает 16 мегапикселей. Укажите --scale 1 или уменьшите исходник.")
    length = 2.5 + simplify * 0.25 if preset == "illustration" else 3.5 + simplify * 0.65
    settings = dict(clustering="color-cluster", hierarchical="stacked", mode="spline",
                    filter_speckle=speckle * scale, color_precision=8, layer_difference=0,
                    corner_threshold=config["corner"], length_threshold=length * scale,
                    max_iterations=10, splice_threshold=45,
                    path_precision=3 if preset == "illustration" else 5, optimize=0)
    if preset == "illustration":
        settings["simplify"] = simplify * scale / 4
    width, height = image.size
    root = ET.Element(f"{{{SVG}}}svg", {"width": str(width), "height": str(height),
                                     "viewBox": f"0 0 {width} {height}"})
    # RGBA Lanczos resampling uses premultiplied alpha, preventing invisible RGB
    # from bleeding into visible edges. The final SVG keeps the original viewport.
    if scale != 1:
        image = image.resize((width * scale, height * scale), Image.Resampling.LANCZOS)
    alpha = image.getchannel("A")
    if alpha.getextrema()[1] == 0:
        return ET.tostring(root, encoding="unicode")
    rgb = image.convert("RGB")
    # Invisible RGB must not pollute palette or create thousands of irrelevant paths.
    hidden = alpha.point(lambda a: 255 if a == 0 else 0)
    sample = image.copy()
    sample.thumbnail((128, 128))
    visible = [p[:3] for p in sample.get_flattened_data() if p[3] > 0]
    if visible:
        from collections import Counter
        rgb.paste(Counter(visible).most_common(1)[0][0], mask=hidden)
    canvas = ET.SubElement(root, f"{{{SVG}}}g", {"transform": f"scale({1 / scale:g})"}) if scale != 1 else root
    group = canvas
    if alpha.getextrema()[0] < 255:
        # Keep an explicit alpha mask: preserve holes and partial opacity without
        # depending on the engine's color-cluster transparency handling.
        defs = ET.SubElement(canvas, f"{{{SVG}}}defs")
        mask = ET.SubElement(defs, f"{{{SVG}}}mask", {
            "id": "alpha", "maskUnits": "userSpaceOnUse", "maskContentUnits": "userSpaceOnUse",
            "x": "0", "y": "0", "width": str(image.width), "height": str(image.height),
            "style": "mask-type:luminance", "color-interpolation": "sRGB",
        })
        mask_settings = dict(settings, filter_speckle=0, mode="pixel", simplify=None, path_precision=5)
        mask.extend(trace(alpha.convert("RGB"), engine, mask_settings))
        group = ET.SubElement(canvas, f"{{{SVG}}}g", {"mask": "url(#alpha)"})
    if preset == "line-art":
        gray = ImageOps.autocontrast(rgb.convert("L"))
        rgb = gray.point(lambda p: 0 if p < 180 else 255).convert("RGB")
        settings["clustering"] = "bw"
        # Binary VTracer outputs foreground only; preserve the original opaque white.
        ET.SubElement(group, f"{{{SVG}}}rect", {"width": str(image.width), "height": str(image.height), "fill": "white"})
    else:
        # Keep the Pillow median-cut palette selected by the A/B experiment.
        # Native quantization is deliberately disabled to avoid a second palette pass.
        rgb = rgb.quantize(colors=colors, method=Image.Quantize.MEDIANCUT,
                           dither=Image.Dither.NONE).convert("RGB")
    paths = trace(rgb, engine, settings)
    if limit_output_colors and preset != "line-art":
        # The engine can average adjacent clusters into additional shades. Honor
        # explicit --colors by snapping only those fills back to the input palette.
        palette = sorted(color for _, color in rgb.getcolors(256))
        remap = {}
        for parent in paths:
            for element in parent.iter():
                fill = element.get("fill", "")
                if re.fullmatch(r"#[0-9A-Fa-f]{6}", fill):
                    if fill not in remap:
                        color = tuple(int(fill[i:i+2], 16) for i in (1, 3, 5))
                        nearest = min(palette, key=lambda p: sum((a-b)**2 for a, b in zip(p, color)))
                        remap[fill] = "#%02x%02x%02x" % nearest
                    element.set("fill", remap[fill])
    group.extend(paths)
    return ET.tostring(root, encoding="unicode")


def vectorize(input_file, output_file, *, preset="auto", colors=None, simplify=None,
              filter_speckle=None, scale=None, optimize=True, verbose=False):
    source, output = Path(input_file), Path(output_file)
    image = load_image(source)
    if source.resolve() == output.resolve():
        raise VectorizeError("Выходной файл совпадает с входным.")
    if output.suffix.lower() != ".svg":
        raise VectorizeError("Выходной файл должен иметь расширение .svg.")
    if preset == "auto":
        preset = choose_preset(image)
    if preset not in PRESETS:
        raise VectorizeError("Неизвестный preset.")
    if colors is not None and (not isinstance(colors, int) or not 2 <= colors <= 256):
        raise VectorizeError("--colors должен быть целым числом от 2 до 256.")
    if simplify is not None and (not math.isfinite(simplify) or not 0 <= simplify <= 10):
        raise VectorizeError("--simplify должен быть числом от 0 до 10.")
    if filter_speckle is not None and (not isinstance(filter_speckle, int) or not 0 <= filter_speckle <= 10000):
        raise VectorizeError("--filter-speckle должен быть целым числом от 0 до 10000.")
    if preset == "line-art" and colors not in (None, 2):
        raise VectorizeError("line-art монохромный: --colors допускает только 2.")
    if scale not in (None, 1, 2):
        raise VectorizeError("--scale должен быть 1 или 2.")
    scale = (2 if preset == "illustration" else 1) if scale is None else scale
    engine = python_dependency("vtracer", "VTracer", "vtracer==1.0.0a4")
    if not hasattr(engine, "Config"):
        raise VectorizeError(f'Несовместимый API VTracer. Выполните "{sys.executable}" -m pip install vtracer==1.0.0a4.')
    node_tools({"action": "check", "optimize": optimize})
    if verbose:
        print(f"preset={preset}; colors={colors or PRESETS[preset]['colors']}; scale={scale}; VTracer -> "
              f"{'SVGO -> ' if optimize else ''}XML/resvg validation", file=sys.stderr)
    svg = build_svg(image, preset, engine, colors, simplify, filter_speckle, scale)
    validate_svg(svg, image.size)
    checked = node_tools({"svg": svg, "optimize": optimize})
    validate_svg(checked["svg"], image.size)
    if not checked["visible"] and image.getchannel("A").getextrema()[1] > 0:
        raise VectorizeError("Невалидный результат: изображение стало пустым. Попробуйте --filter-speckle 0.")
    if image.getchannel("A").getextrema()[0] == 255 and not checked["opaque"]:
        raise VectorizeError("Невалидный результат: в непрозрачном изображении появились прозрачные щели.")
    # All checks finish before touching an existing destination. Same-volume replace.
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", newline="\n",
                                         dir=output.parent, prefix=".vectorize-", suffix=".svg", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(checked["svg"] + "\n")
        os.replace(temporary, output)
    finally:
        if temporary and temporary.exists():
            temporary.unlink()
    return {"output": str(output.resolve()), "preset": preset, "bytes": output.stat().st_size,
            "optimized": optimize, "scale": scale, "render_max_difference": checked["maxDifference"]}


def main(argv=None):
    parser = argparse.ArgumentParser(prog="vectorize", description="Локальная PNG/JPG/WebP -> SVG векторизация: VTracer, SVGO, validation.")
    parser.add_argument("input", type=Path, help="входной PNG, JPG/JPEG или WebP")
    parser.add_argument("-o", "--output", required=True, type=Path, help="путь к выходному SVG")
    parser.add_argument("--preset", choices=["auto", *PRESETS], default="auto", help="профиль; по умолчанию auto")
    parser.add_argument("--colors", type=int, help="максимум цветов палитры 2..256 (без оттенков alpha-маски)")
    parser.add_argument("--simplify", type=float, help="упрощение 0..10: больше — меньше деталей; шкала VTracer length_threshold")
    parser.add_argument("--filter-speckle", type=int, help="порог мелких пятен в пикселях исходника; 0 сохраняет мелкие детали")
    parser.add_argument("--scale", type=int, choices=(1, 2), help="масштаб трассировки: illustration=2 (Lanczos), остальные=1; размеры SVG сохраняются")
    parser.add_argument("--no-optimize", action="store_true", help="пропустить SVGO; XML и render validation остаются")
    parser.add_argument("--verbose", action="store_true", help="показать preset и стадии pipeline")
    args = parser.parse_args(argv)
    try:
        result = vectorize(args.input, args.output, preset=args.preset, colors=args.colors,
                           simplify=args.simplify, filter_speckle=args.filter_speckle, scale=args.scale,
                           optimize=not args.no_optimize, verbose=args.verbose)
    except (VectorizeError, OSError) as error:
        print(f"vectorize: {error}", file=sys.stderr)
        return 1
    print(f"OK: {result['output']} (preset={result['preset']}, {result['bytes']} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
