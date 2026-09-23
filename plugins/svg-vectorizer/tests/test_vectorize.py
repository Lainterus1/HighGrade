import base64
import io
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

from PIL import Image
from scripts import vectorize as tool
from tests.fixtures import logo, line_art, illustration


class PipelineTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="svg-vectorizer-tests-")
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.source = self.folder / "вход с пробелами.png"
        logo().save(self.source)
        self.output = self.folder / "выход с пробелами.svg"

    def convert(self, image=None, preset="illustration", **kwargs):
        if image is not None:
            image.save(self.source)
        report = tool.vectorize(self.source, self.output, preset=preset, **kwargs)
        svg = self.output.read_text(encoding="utf-8")
        size = image.size if image else logo().size
        root = tool.validate_svg(svg, size)
        rendered = tool.node_tools({"svg": svg, "optimize": False, "preview": True})
        raster = Image.open(io.BytesIO(base64.b64decode(rendered["png"]))).convert("RGBA")
        return report, root, raster

    def test_png_svg_and_alpha(self):
        report, root, image = self.convert(preset="logo")
        self.assertTrue(report["optimized"])
        self.assertEqual(image.size, (192, 128))
        self.assertEqual(image.getpixel((0, 0))[3], 0)
        self.assertEqual(image.getpixel((64, 64))[3], 0)  # transparent hole
        self.assertEqual(image.getpixel((32, 64))[3], 255)
        self.assertLessEqual(abs(image.getpixel((145, 64))[3] - 128), 2)
        self.assertTrue(root.findall(f".//{{{tool.SVG}}}path"))
        self.assertNotIn("<image", self.output.read_text())

    def test_jpg(self):
        self.source = self.folder / "input.jpg"
        illustration().save(self.source, quality=95)
        report = tool.vectorize(self.source, self.output)
        tool.validate_svg(self.output.read_text(), (128, 96))
        self.assertEqual(report["preset"], "illustration")

    def test_webp_transparency(self):
        self.source = self.folder / "input.webp"
        logo().save(self.source, lossless=True)
        _, _, image = self.convert()
        self.assertEqual(image.getpixel((64, 64))[3], 0)
        self.assertLessEqual(abs(image.getpixel((145, 64))[3] - 128), 2)

    def test_every_preset_renders(self):
        for preset in tool.PRESETS:
            with self.subTest(preset=preset):
                source = line_art() if preset == "line-art" else logo()
                _, _, raster = self.convert(source, preset=preset)
                self.assertIsNotNone(raster.getbbox())
                if preset != "line-art":
                    self.assertEqual(raster.getpixel((64, 64))[3], 0)

    def test_illustration_small_detail_survives(self):
        _, _, image = self.convert(illustration())
        red, green, blue, alpha = image.getpixel((91, 13))
        self.assertGreater(red, green + 70)
        self.assertEqual(alpha, 255)

    def test_line_art_preserves_thin_line_removes_isolated_noise(self):
        _, _, image = self.convert(line_art(), preset="line-art")
        self.assertLess(image.getpixel((60, 80))[0], 100)
        self.assertGreater(image.getpixel((120, 8))[0], 240)
        self.assertEqual(image.getpixel((0, 0)), (255, 255, 255, 255))

    def test_alpha_levels_and_invisible_rgb(self):
        source = Image.new("RGBA", (96, 32), (200, 100, 90, 0))
        for x in range(96):
            for y in range(32):
                source.putpixel((x, y), (40, 120, 210, [0, 64, 128, 192, 255, 0][x // 16]))
        _, _, image = self.convert(source)
        for x, alpha in [(8, 0), (24, 64), (40, 128), (56, 192), (72, 255), (88, 0)]:
            self.assertLessEqual(abs(image.getpixel((x, 16))[3] - alpha), 2)

    def test_palette_transparent_png(self):
        source = Image.new("P", (32, 32), 0)
        source.putpalette([0, 0, 0, 255, 0, 0] + [0, 0, 0] * 254)
        source.info["transparency"] = 0
        for x in range(8, 24):
            for y in range(8, 24):
                source.putpixel((x, y), 1)
        _, _, image = self.convert(source)
        self.assertEqual(image.getpixel((0, 0))[3], 0)
        self.assertEqual(image.getpixel((16, 16))[3], 255)

    def test_empty_transparent_image(self):
        _, _, image = self.convert(Image.new("RGBA", (32, 32)))
        self.assertIsNone(image.getbbox())

    def test_no_optimize_and_overrides(self):
        report, _, _ = self.convert(colors=4, simplify=0, filter_speckle=0, optimize=False)
        self.assertFalse(report["optimized"])

    def test_color_limit(self):
        _, root, _ = self.convert(illustration(), colors=3)
        fills = {p.get("fill") for p in root.findall(f".//{{{tool.SVG}}}path")}
        self.assertLessEqual(len(fills), 3)

    def test_deterministic_output(self):
        self.convert()
        first = self.output.read_bytes()
        self.convert()
        self.assertEqual(first, self.output.read_bytes())

    def test_auto_selection(self):
        self.assertEqual(tool.choose_preset(line_art().convert("RGBA")), "line-art")
        self.assertEqual(tool.choose_preset(logo()), "icon")
        self.assertEqual(tool.choose_preset(logo().resize((768, 512))), "logo")
        self.assertEqual(tool.choose_preset(illustration().convert("RGBA")), "illustration")

    def test_missing_input(self):
        with self.assertRaisesRegex(tool.VectorizeError, "отсутствует"):
            tool.vectorize(self.folder / "missing.png", self.output)
        self.assertFalse(self.output.exists())

    def test_unsupported_format(self):
        self.source = self.folder / "input.bmp"
        logo().save(self.source)
        with self.assertRaisesRegex(tool.VectorizeError, "Неподдерживаемый формат"):
            tool.vectorize(self.source, self.output)

    def test_disguised_format_and_corrupt_input(self):
        logo().convert("RGB").save(self.source, format="BMP")
        with self.assertRaisesRegex(tool.VectorizeError, "не соответствует"):
            tool.vectorize(self.source, self.output)
        self.source.write_bytes(b"not an image")
        with self.assertRaisesRegex(tool.VectorizeError, "прочитать"):
            tool.vectorize(self.source, self.output)

    def test_animation_rejected(self):
        logo().save(self.source, save_all=True, append_images=[logo().transpose(Image.Transpose.FLIP_LEFT_RIGHT)])
        with self.assertRaisesRegex(tool.VectorizeError, "Анимация"):
            tool.vectorize(self.source, self.output)

    def test_missing_vtracer_has_install_command(self):
        real_import = tool.importlib.import_module
        def missing(name):
            if name == "vtracer":
                raise ModuleNotFoundError(name)
            return real_import(name)
        with patch.object(tool.importlib, "import_module", side_effect=missing):
            with self.assertRaisesRegex(tool.VectorizeError, "pip install vtracer==1.0.0a4"):
                tool.vectorize(self.source, self.output)
        self.assertFalse(self.output.exists())

    def test_missing_svgo_real_bridge(self):
        bridge = self.folder / "isolated.mjs"
        bridge.write_text(tool.BRIDGE.read_text(encoding="utf-8").replace(
            "dependency('svgo',", "dependency('svg-vectorizer-deliberately-missing',"), encoding="utf-8")
        with patch.object(tool, "BRIDGE", bridge):
            with self.assertRaisesRegex(tool.VectorizeError, "SVGO.*npm ci"):
                tool.vectorize(self.source, self.output)
        self.assertFalse(self.output.exists())

    def test_missing_node(self):
        with patch.object(tool.shutil, "which", return_value=None):
            with self.assertRaisesRegex(tool.VectorizeError, "Node.js не установлен"):
                tool.vectorize(self.source, self.output)

    def test_engine_failure_preserves_existing_output(self):
        self.output.write_text("keep me")
        import vtracer
        with patch.object(vtracer, "Config", side_effect=RuntimeError("fixture failure")):
            with self.assertRaisesRegex(tool.VectorizeError, "Ошибка векторизации.*fixture failure"):
                tool.vectorize(self.source, self.output)
        self.assertEqual(self.output.read_text(), "keep me")

    def test_validation_failure_preserves_existing_output(self):
        self.output.write_text("keep me")
        with patch.object(tool, "build_svg", return_value="<broken>"):
            with self.assertRaisesRegex(tool.VectorizeError, "Невалидный SVG"):
                tool.vectorize(self.source, self.output)
        self.assertEqual(self.output.read_text(), "keep me")

    def test_opaque_source_cannot_develop_transparent_gaps(self):
        Image.new("RGB", (32, 32), "red").save(self.source)
        self.output.write_text("keep me")
        svg = f'<svg xmlns="{tool.SVG}" width="32" height="32" viewBox="0 0 32 32"><rect width="16" height="32" fill="red"/></svg>'
        with patch.object(tool, "build_svg", return_value=svg):
            with self.assertRaisesRegex(tool.VectorizeError, "прозрачные щели"):
                tool.vectorize(self.source, self.output, preset="illustration")
        self.assertEqual(self.output.read_text(), "keep me")

    def test_old_engine_api_has_update_command(self):
        real_import = tool.importlib.import_module
        with patch.object(tool.importlib, "import_module", side_effect=lambda name:
                          SimpleNamespace() if name == "vtracer" else real_import(name)):
            with self.assertRaisesRegex(tool.VectorizeError, "Несовместимый API.*pip install vtracer==1.0.0a4"):
                tool.vectorize(self.source, self.output)
        self.assertFalse(self.output.exists())

    def test_illustration_scale_preserves_output_dimensions_and_opacity(self):
        for scale in (1, 2):
            with self.subTest(scale=scale):
                _, _, rendered = self.convert(illustration(), scale=scale)
                self.assertEqual(rendered.size, (128, 96))
                self.assertEqual(rendered.getchannel("A").getextrema(), (255, 255))

    def test_scaled_pixel_budget_requires_explicit_smaller_scale(self):
        illustration().save(self.source)
        with patch.object(tool, "MAX_PIXELS", 20_000):
            with self.assertRaisesRegex(tool.VectorizeError, "--scale 1"):
                tool.vectorize(self.source, self.output, preset="illustration")
            self.assertFalse(self.output.exists())
            tool.vectorize(self.source, self.output, preset="illustration", scale=1)
        tool.validate_svg(self.output.read_text(), (128, 96))

    def test_invalid_svg_rejected(self):
        for svg in ["<broken>", "<html/>", '<svg xmlns="http://www.w3.org/2000/svg"/>']:
            with self.subTest(svg=svg), self.assertRaises(tool.VectorizeError):
                tool.validate_svg(svg, (32, 32))

    def test_malformed_path_rejected_even_with_valid_xml(self):
        for data in ["M0 0 L", "M0 0 C1 2", "M0 0 L1e999 2", "M0 0 BAD", "M0 0 A1 1 0 3 1 2 2"]:
            with self.subTest(data=data), self.assertRaises(tool.VectorizeError):
                tool.validate_svg(f'<svg xmlns="{tool.SVG}" width="32" height="32" viewBox="0 0 32 32"><path d="{data}"/></svg>', (32, 32))

    def test_no_optimize_does_not_require_svgo_but_requires_renderer(self):
        bridge = self.folder / "no-svgo.mjs"
        renderer = (tool.ROOT / "node_modules/@resvg/resvg-js/index.js").as_uri()
        bridge.write_text(tool.BRIDGE.read_text(encoding="utf-8").replace(
            "dependency('svgo',", "dependency('svg-vectorizer-deliberately-missing',").replace(
            "dependency('@resvg/resvg-js',", f"dependency('{renderer}',"), encoding="utf-8")
        with patch.object(tool, "BRIDGE", bridge):
            tool.vectorize(self.source, self.output, optimize=False)
        bridge.write_text(bridge.read_text(encoding="utf-8").replace(
            f"dependency('{renderer}',", "dependency('svg-vectorizer-missing-renderer',"), encoding="utf-8")
        with patch.object(tool, "BRIDGE", bridge):
            with self.assertRaisesRegex(tool.VectorizeError, "renderer.*npm ci"):
                tool.vectorize(self.source, self.output, optimize=False)

    def test_flat_white_and_black(self):
        for color in ("white", "black"):
            with self.subTest(color=color):
                _, _, image = self.convert(Image.new("RGB", (32, 32), color), preset="line-art")
                self.assertEqual(image.getpixel((16, 16)), (255, 255, 255, 255) if color == "white" else (0, 0, 0, 255))

    def test_invalid_arguments(self):
        for kwargs in [dict(colors=1), dict(simplify=float("nan")), dict(simplify=11),
                       dict(filter_speckle=-1), dict(preset="line-art", colors=8), dict(scale=3)]:
            with self.subTest(kwargs=kwargs), self.assertRaises(tool.VectorizeError):
                tool.vectorize(self.source, self.output, **kwargs)

    def test_exif_orientation(self):
        self.source = self.folder / "rotated.jpg"
        exif = Image.Exif()
        exif[274] = 6
        illustration().save(self.source, exif=exif)
        tool.vectorize(self.source, self.output)
        tool.validate_svg(self.output.read_text(), (96, 128))

    def test_cli_help_and_real_conversion(self):
        script = tool.ROOT / "scripts" / "vectorize.py"
        help_result = subprocess.run([sys.executable, str(script), "--help"], capture_output=True)
        self.assertEqual(help_result.returncode, 0)
        result = subprocess.run([sys.executable, str(script), str(self.source), "-o", str(self.output),
                                 "--preset", "icon", "--verbose"], capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", errors="replace"))
        self.assertTrue(self.output.is_file())


if __name__ == "__main__":
    unittest.main()
