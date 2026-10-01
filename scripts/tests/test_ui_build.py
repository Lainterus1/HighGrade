"""Run the real build.rs in a tiny offline crate, without rebuilding the product."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class UiBuildTests(unittest.TestCase):
    def test_release_requires_complete_bundle_but_debug_allows_absent_dist(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'src/ui').mkdir(parents=True)
            shutil.copy2(ROOT / 'build.rs', root / 'build.rs')
            shutil.copy2(ROOT / 'src/ui/bundle.rs', root / 'src/ui/bundle.rs')
            package = tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf-8'))
            version = package['package']['version']
            dependencies = (ROOT / 'Cargo.toml').read_text().split('[build-dependencies]\n')[1].split('\n[')[0]
            (root / 'Cargo.toml').write_text(
                f'[package]\nname="ui-build-fixture"\nversion="{version}"\nedition="2024"\n'
                f'[build-dependencies]\n{dependencies}\n')
            (root / 'src/main.rs').write_text('fn main() {}')
            environment = dict(os.environ, CARGO_TARGET_DIR=str(root / 'target'))
            environment.pop('HIGHGRADE_SOURCE_SHA', None)

            def build(release=False, success=True, sha=None):
                env = dict(environment)
                if sha:
                    env['HIGHGRADE_SOURCE_SHA'] = sha
                result = subprocess.run(['cargo', 'build', '--offline', *(['--release'] if release else [])],
                                        cwd=root, env=env, capture_output=True, text=True, encoding='utf-8')
                self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
                if not success:
                    self.assertIn('UiBundleInvalid', result.stderr)

            build()  # Deliberate CLI-only developer build.
            build(release=True, success=False)
            dist = root / 'ui/dist'
            dist.mkdir(parents=True)
            (dist / 'index.html').write_text('incomplete')
            build(success=False)
            assets = ['index.html', 'fonts/Manrope.ttf', 'brand/mark.svg', 'brand/wordmark.svg',
                      'licenses/Manrope-OFL.txt', 'licenses/Lucide-LICENSE.txt', 'licenses/packages.json',
                      'assets/app.js', 'assets/app.css']
            for asset in assets:
                destination = dist / asset
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_text('fixture ' + asset)
            manifest = {'schema_version': 1, 'api_version': '2', 'cli_version': version,
                        'source_sha': 'a' * 40,
                        'files': {asset: hashlib.sha256((dist / asset).read_bytes()).hexdigest() for asset in assets}}

            def save():
                (dist / 'highgrade-ui.json').write_text(json.dumps(manifest))

            save()
            build(release=True, sha='a' * 40)
            (dist / 'assets/app.js').write_text('corrupt')
            build(release=True, success=False)
            (dist / 'assets/app.js').write_text('fixture assets/app.js')
            manifest['cli_version'] = '999.0.0'
            save()
            build(release=True, success=False)
            manifest['cli_version'] = version
            save()
            build(release=True, success=False, sha='b' * 40)
            (dist / 'assets/app.css').unlink()
            manifest['files'].pop('assets/app.css')
            save()
            build(release=True, success=False)
