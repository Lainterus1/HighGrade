// One short-lived process: dependency preflight, optimization and actual rendering.
import fs from 'node:fs';

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

async function dependency(name, label) {
  try {
    return await import(name);
  } catch (error) {
    fail(`${label} не установлен или не загружается. В корне плагина выполните npm ci --ignore-scripts. ${error.message}`);
  }
}

try {
  const request = JSON.parse(fs.readFileSync(0, 'utf8'));
  const optimizer = request.optimize ? await dependency('svgo', 'SVGO') : null;
  const { Resvg } = await dependency('@resvg/resvg-js', 'SVG renderer (resvg)');
  if (request.action === 'check') {
    process.stdout.write('{}');
    process.exit(0);
  }
  const render = (svg) => {
    const result = new Resvg(svg, {font: {loadSystemFonts: false}}).render();
    return {pixels: result.pixels, width: result.width, height: result.height,
      png: request.preview ? result.asPng().toString('base64') : undefined};
  };
  const before = render(request.svg);
  let svg = request.svg;
  if (optimizer) {
    // Explicit list: no removeHiddenElems/removeUselessDefs/removeViewBox or forced merge.
    svg = optimizer.optimize(svg, {
      multipass: false,
      plugins: [
        'removeDoctype', 'removeXMLProcInst', 'removeComments', 'removeMetadata',
        'removeEditorsNSData', 'cleanupAttrs', 'convertColors',
        {name: 'convertPathData', params: {
          floatPrecision: 5, applyTransforms: false,
          makeArcs: {threshold: 0, tolerance: 0},
          straightCurves: false, convertToQ: false, smartArcRounding: false,
        }},
        'sortAttrs',
      ],
    }).data;
  }
  const after = optimizer ? render(svg) : before;
  if (before.width !== after.width || before.height !== after.height) {
    fail('Невалидный результат: SVGO изменил размеры изображения.');
  }
  // Reject any material rasterized change instead of silently accepting damaged artwork.
  let maxDifference = 0;
  for (let i = 0; i < before.pixels.length; i++) {
    maxDifference = Math.max(maxDifference, Math.abs(before.pixels[i] - after.pixels[i]));
  }
  if (maxDifference > 2) {
    fail(`Невалидный результат: SVGO изменил отрисовку (разница канала ${maxDifference}/255). Попробуйте --no-optimize.`);
  }
  let visible = false;
  let opaque = true;
  for (let i = 3; i < after.pixels.length; i += 4) {
    visible ||= after.pixels[i] > 0;
    opaque &&= after.pixels[i] === 255;
  }
  process.stdout.write(JSON.stringify({svg, width: after.width, height: after.height,
    visible, opaque, maxDifference, png: after.png}));
} catch (error) {
  fail(`Ошибка SVG processing/validation: ${error.message}`);
}
