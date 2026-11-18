async (page) => {
  const flagOffReqs = [];
  const ewmReqs = [];
  let bucket = flagOffReqs;
  page.on("request", (req) => {
    bucket.push(req.url());
  });
  page.setDefaultTimeout(120000);
  await page.goto("http://127.0.0.1:8800/", { waitUntil: "networkidle", timeout: 60000 });
  await page.waitForTimeout(2500);
  const flagOff = await page.evaluate(() => ({
    status: document.getElementById("native-world-status")?.textContent || "",
    fallback: document.getElementById("native-fallback")?.textContent || "",
    receipt: window.__webizenNativeWorld || null,
    href: location.href,
  }));
  await page.screenshot({
    path: "C:/github/webizen-website/evidence/ewm/flag-off-fail-closed.png",
    fullPage: true,
  });
  bucket = ewmReqs;
  await page.goto("http://127.0.0.1:8800/?ewm=1", { waitUntil: "domcontentloaded", timeout: 60000 });
  await page.waitForFunction(() => {
    const r = window.__webizenNativeWorld;
    return !!(r && r.ewm_streaming === true && r.scene_graph && r.scene_graph.format === "q42");
  }, { timeout: 120000 });
  await page.waitForTimeout(8000);
  const ewm = await page.evaluate(() => {
    const r = window.__webizenNativeWorld || {};
    const ids = (r.assets || []).map((a) => a.id);
    return {
      ewm_streaming: r.ewm_streaming,
      streaming: r.streaming,
      scene_graph: r.scene_graph,
      asset_count: (r.assets || []).length,
      asset_bytes: r.asset_bytes,
      organs_loaded: r.upload && r.upload.organs_loaded,
      organs_refused: r.upload && r.upload.organs_refused,
      upload_triangles: r.upload && r.upload.total_triangles,
      camper_parts: ids.filter((id) => String(id).startsWith("vintage-windup-camper--")).length,
      has_scene_json: false,
      status: document.getElementById("native-world-status")?.textContent || "",
    };
  });
  await page.screenshot({
    path: "C:/github/webizen-website/evidence/ewm/ewm-1-scene-q42.png",
  });
  const classify = (urls) => ({
    scene_json: urls.filter((u) => u.includes("scene.json") || u.includes("scene-mobile.json")),
    scene_q42: urls.filter((u) => u.includes("scene.q42")),
    water: urls.filter((u) => u.includes("water.bin")),
    ten_d: urls.filter((u) => u.includes(".10d") || u.includes("/packages/")),
  });
  return {
    flagOff,
    flagOffNetwork: classify(flagOffReqs),
    ewm,
    ewmNetwork: classify(ewmReqs),
  };
}
