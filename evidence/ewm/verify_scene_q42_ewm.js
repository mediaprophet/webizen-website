async (page) => {
  const reqs = [];
  page.on("request", (req) => reqs.push(req.url()));
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.setDefaultTimeout(180000);
  await page.goto("http://127.0.0.1:8800/?ewm=1", {
    waitUntil: "domcontentloaded",
    timeout: 60000,
  });
  const started = Date.now();
  while (Date.now() - started < 150000) {
    const state = await page.evaluate(() => ({
      status: document.getElementById("native-world-status")?.textContent || "",
      fallback: document.getElementById("native-fallback")?.textContent || "",
      receipt: window.__webizenNativeWorld || null,
    }));
    if (state.receipt && state.receipt.ewm_streaming === true) {
      await page.screenshot({
        path: "C:/github/webizen-website/evidence/ewm/ewm-1-scene-q42.png",
      });
      const ids = (state.receipt.assets || []).map((a) => a.id);
      return {
        ok: true,
        status: state.status,
        ewm_streaming: state.receipt.ewm_streaming,
        streaming: state.receipt.streaming,
        scene_graph: state.receipt.scene_graph,
        asset_count: (state.receipt.assets || []).length,
        asset_bytes: state.receipt.asset_bytes,
        organs_loaded: state.receipt.upload && state.receipt.upload.organs_loaded,
        organs_refused: state.receipt.upload && state.receipt.upload.organs_refused,
        upload_triangles: state.receipt.upload && state.receipt.upload.total_triangles,
        camper_parts: ids.filter((id) => String(id).startsWith("vintage-windup-camper--")).length,
        scene_json_hits: reqs.filter((u) => u.includes("scene.json")),
        scene_q42_hits: reqs.filter((u) => u.includes("scene.q42")),
        water_hits: reqs.filter((u) => u.includes("water.bin")),
        errors,
      };
    }
    if (
      state.fallback &&
      state.status &&
      state.status !== "Opening the native living world…" &&
      state.status.indexOf("Opening the streamed") === -1 &&
      state.status.indexOf("Opening native material") === -1 &&
      state.status.indexOf("Opening the landscape") === -1
    ) {
      return {
        ok: false,
        status: state.status,
        fallback: state.fallback,
        scene_json_hits: reqs.filter((u) => u.includes("scene.json")),
        scene_q42_hits: reqs.filter((u) => u.includes("scene.q42")),
        water_hits: reqs.filter((u) => u.includes("water.bin")),
        errors,
      };
    }
    await page.waitForTimeout(500);
  }
  const last = await page.evaluate(() => ({
    status: document.getElementById("native-world-status")?.textContent || "",
    fallback: document.getElementById("native-fallback")?.textContent || "",
  }));
  return { ok: false, timeout: true, last, errors, reqs: reqs.slice(-20) };
}
