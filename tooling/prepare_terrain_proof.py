"""Prepare a separate native measured-data exhibit, with explicit model scale."""
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
source=(ROOT/'site/world/living-world.vibe').read_text(encoding='utf8')
source=source.replace('default_locale: "en-AU",','default_locale: "en-AU",\n        scene_mode: "reference-model",\n        scene_title: "A landscape you can inspect",\n        scene_description: "Real Canberra reference terrain, shown at 1:30 scale with no vertical exaggeration. Source elevations are 541 to 677 metres. This public reference landscape is independent of the personal camping journey. Neutral relief shading is not satellite imagery.",')
source=source.replace('camera: {yaw: 0.63275, pitch: 0.17555, zoom: 18.8915, target: [-22.0, 5.490921689, 28.0]}','camera: {yaw: 0.55, pitch: 0.65, zoom: 105.0, target: [0.0, 1.8, 0.0]}')
source=source.replace('3.0, 150.0','3.0, 500.0')
(ROOT/'site/world/terrain-proof.vibe').write_text(source,encoding='utf8')
html=(ROOT/'site/native.html').read_text(encoding='utf8')
html=html.replace('<canvas id="native-world"','<canvas data-scene="/data/publications/aaa/terrain-model-v2/scene.json" data-vibe="/world/terrain-proof.vibe" id="native-world"')
html=html.replace('<title>Webizen — The living world</title>','<title>Webizen — Canberra terrain evidence</title>')
html=html.replace('<a href="/lab/">Tests &amp; benchmarks</a>','<a href="/aaa-proof.html">Return to the living world</a>')
html=html.replace('<main id="native-world-ui"></main>','<main id="native-world-ui"></main><aside class="terrain-source"><b>Canberra · public reference landscape</b><p>Mapzen Terrain Tiles; Australian terrain © Commonwealth of Australia (Geoscience Australia) 2017. Source rasters, attribution and the native model remain inspectable.</p><a href="/data/publications/aaa/terrain-model-v2/model-manifest.json">Inspect scale, source and production evidence</a> · <a href="/data/publications/aaa/terrain/source-tiles.qdem" download>Download retained source tiles</a><p>Drag to orbit · scroll to zoom · open the laboratory to query the data.</p></aside>')
(ROOT/'site/terrain-proof.html').write_text(html,encoding='utf8')
print('Prepared native reference terrain exhibit')
