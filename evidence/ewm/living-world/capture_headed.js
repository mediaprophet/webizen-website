async (page) => {
  const outDir = "C:/github/webizen-website/evidence/ewm/living-world";

  const reqs = [];
  const errors = [];
  page.on("request", (req) => reqs.push(req.url()));
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.setDefaultTimeout(180000);

  async function state() {
    return page.evaluate(() => {
      const r = window.__webizenNativeWorld || null;
      const ids = ((r && r.assets) || []).map((a) => a.id || a.scene_id || "");
      const camper = ids.filter((id) => String(id).startsWith("vintage-windup-camper--"));
      const base = (r && r.ewm_live_place_base) || [];
      return {
        status: document.getElementById("native-world-status")?.textContent || "",
        fallback: document.getElementById("native-fallback")?.textContent || "",
        href: location.href,
        ewm_streaming: r && r.ewm_streaming,
        streaming: r && r.streaming,
        selected_place: r && r.selected_place,
        ewm_live_place: r && r.ewm_live_place,
        ewm_live_place_base: base,
        ewm_unload: r && r.ewm_unload,
        camp: r && r.camp,
        asset_count: ids.length,
        camper_parts: camper.length,
        camper_ids: camper.slice(0, 8),
        organs_loaded: r && r.upload && r.upload.organs_loaded,
        ewm_live_bytes: r && r.ewm_live_bytes,
        ewm_live_triangles: r && r.ewm_live_triangles,
        camp_buttons: Array.from(
          document.querySelectorAll("#native-camp-controls button")
        ).map((b) => b.textContent),
        place_buttons: Array.from(
          document.querySelectorAll("#native-place-list button")
        ).map((b) => b.textContent),
        panel_hidden: document.getElementById("native-world-ui")?.hasAttribute("hidden") || false,
      };
    });
  }

  async function waitUntil(pred, ms) {
    const started = Date.now();
    let last = null;
    while (Date.now() - started < ms) {
      last = await state();
      if (pred(last)) return last;
      await page.waitForTimeout(400);
    }
    return last;
  }

  async function showPanel() {
    const hidden = await page.evaluate(
      () => document.getElementById("native-world-ui")?.hasAttribute("hidden") || false
    );
    if (hidden) {
      const toggle = page.getByRole("button", { name: /Show panel|Hide panel/i });
      if (await toggle.count()) {
        await toggle.first().click();
      }
    }
  }

  async function clickNamed(name) {
    await showPanel();
    const btn = page.getByRole("button", { name, exact: true });
    await btn.click({ timeout: 15000 });
  }

  async function shot(name) {
    const dest = outDir + "/" + name;
    await page.screenshot({ path: dest, fullPage: true });
    return dest;
  }

  async function boot() {
    await page.goto("http://127.0.0.1:8800/?ewm=1", {
      waitUntil: "domcontentloaded",
      timeout: 60000,
    });
    const ready = await waitUntil(
      (s) =>
        s.ewm_streaming === true &&
        (s.camp_buttons || []).includes("Pack up") &&
        (s.place_buttons || []).some((t) => /living river/i.test(t)),
      150000
    );
    await showPanel();
    return ready;
  }

  const hitch = { ok: false };
  const unload = { ok: false };
  const unhitch = { ok: false };
  const wasd = { ok: false };

  const boot1 = await boot();
  hitch.boot = {
    ewm_streaming: boot1 && boot1.ewm_streaming,
    status: boot1 && boot1.status,
    camp_buttons: boot1 && boot1.camp_buttons,
    place_buttons: boot1 && boot1.place_buttons,
    camper_parts: boot1 && boot1.camper_parts,
  };
  if (boot1 && boot1.ewm_streaming) {
    await clickNamed("Pack up");
    hitch.after_pack = await waitUntil(
      (s) => s.camp && (s.camp.gate === true || s.camp.travel === 1),
      20000
    );
    await clickNamed("The living river");
    hitch.after_travel = await waitUntil(
      (s) =>
        s.selected_place === "river" ||
        s.ewm_live_place === "river" ||
        /travelling/i.test(s.status || ""),
      12000
    );
    hitch.screenshot = await shot("hitch-river.png");
    hitch.ok = true;
    hitch.state = await state();

    const settled = await waitUntil((s) => {
      const base = s.ewm_live_place_base || [];
      const omitted =
        s.ewm_live_place === "river" &&
        !base.some((id) => String(id).startsWith("vintage-windup-camper--"));
      return omitted || (s.ewm_unload === "omit-and-rebuild" && s.ewm_live_place === "river");
    }, 90000);
    unload.screenshot = await shot("place-unload.png");
    unload.state = settled;
    unload.ok = !!(
      settled &&
      settled.ewm_live_place === "river" &&
      !(settled.ewm_live_place_base || []).some((id) =>
        String(id).startsWith("vintage-windup-camper--")
      )
    );
  }

  const boot2 = await boot();
  unhitch.boot = {
    ewm_streaming: boot2 && boot2.ewm_streaming,
    camper_parts: boot2 && boot2.camper_parts,
  };
  if (boot2 && boot2.ewm_streaming) {
    await clickNamed("Pack up");
    await waitUntil((s) => s.camp && s.camp.gate === true, 20000);
    await clickNamed("Unhitch");
    unhitch.after_unhitch = await waitUntil(
      (s) => s.camp && s.camp.unhitched === true,
      12000
    );
    await clickNamed("The living river");
    unhitch.after_travel = await waitUntil(
      (s) =>
        s.selected_place === "river" ||
        /car is travelling/i.test(s.status || "") ||
        /travelling/i.test(s.status || ""),
      12000
    );
    unhitch.screenshot = await shot("unhitch-river.png");
    unhitch.state = await state();
    unhitch.ok = true;

    const canvas = page.locator("#native-world-canvas, canvas").first();
    if (await canvas.count()) {
      await canvas.click({ timeout: 5000 }).catch(() => {});
      await page.keyboard.down("ArrowRight");
      await page.waitForTimeout(1500);
      await page.keyboard.up("ArrowRight");
      await page.keyboard.down("KeyW");
      await page.waitForTimeout(800);
      await page.keyboard.up("KeyW");
      wasd.screenshot = await shot("unhitch-wasd.png");
      wasd.state = await state();
      wasd.ok = true;
    }
  }

  return {
    hitch,
    unload,
    unhitch,
    wasd,
    errors: errors.slice(0, 20),
    range_hits: reqs.filter((u) => u.includes("/packages/") || u.includes("/production/") || u.includes("/data/ewm/places/")).length,
    scene_json_hits: reqs.filter((u) => u.includes("scene.json")).length,
  };
}
