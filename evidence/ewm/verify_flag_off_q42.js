async (page) => {
  const reqs = [];
  page.on("request", (req) => reqs.push(req.url()));
  page.setDefaultTimeout(60000);
  await page.goto("http://127.0.0.1:8800/", { waitUntil: "networkidle", timeout: 60000 });
  await page.waitForTimeout(2000);
  const state = await page.evaluate(() => ({
    status: document.getElementById("native-world-status")?.textContent || "",
    fallback: document.getElementById("native-fallback")?.textContent || "",
    receipt: window.__webizenNativeWorld || null,
    href: location.href,
  }));
  await page.screenshot({
    path: "C:/github/webizen-website/evidence/ewm/flag-off-fail-closed.png",
    fullPage: true,
  });
  return {
    state,
    scene_json: reqs.filter((u) => u.includes("scene.json") || u.includes("scene-mobile.json")),
    scene_q42: reqs.filter((u) => u.includes("scene.q42")),
    water: reqs.filter((u) => u.includes("water.bin")),
    ten_d: reqs.filter((u) => u.includes(".10d")),
    packages: reqs.filter((u) => u.includes("/packages/")),
  };
}
