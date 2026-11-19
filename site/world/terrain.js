// One deterministic terrain definition shared with site/world/living-world.vibe and production/terrain-v2/.
function clamp(v, lo, hi) {
  return Math.min(hi, Math.max(lo, v));
}

function smoothstep(edge0, edge1, x) {
  const t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}

export function riverX(z) {
  const oz = smoothstep(65.0, 125.0, Math.abs(z));
  return Math.sin(z * 0.044) * 10.0
    + Math.sin(z * 0.095) * 3.0
    + oz * (Math.sin(z * 0.0175) * 26.0 + Math.sin(z * 0.0085 + 0.7) * 14.0);
}

export function noise(x, z) {
  return Math.sin(x * 0.11 + z * 0.07) * 0.5
    + Math.sin(x * 0.28 - z * 0.16) * 0.22
    + Math.cos(z * 0.39 + x * 0.2) * 0.12
    + Math.sin(x * 0.73 + z * 0.61) * 0.05;
}

export function coreHeightAt(x, z) {
  const d = Math.abs(x - riverX(z));
  const bank = 1.0 - Math.exp(-Math.pow(d / 9.0, 2.0));
  const bankCh = 1.0 - Math.exp(-Math.pow(d / 6.5, 2.0));
  const valley = 0.35 + bank * (2.4 + Math.max(0.0, Math.abs(x) - 15.0) * 0.22);
  const mountain = Math.pow(Math.max(0.0, (-z - 25.0) / 65.0), 1.4)
    * (7.0 + 12.0 * Math.pow(Math.sin(x * 0.055 + 1.2), 2.0));
  const channelGap = 1.0 - smoothstep(-34.0, -48.0, z) * (1.0 - bankCh);
  const mMasked = mountain * channelGap;
  return valley + mMasked + noise(x, z) * bank * (1.4 + mMasked * 0.17)
    - Math.exp(-d * d / 16.0) * 1.4;
}

export function outerHeightAt(x, z) {
  const rx = riverX(z);
  const dx = x - rx;
  const d = Math.abs(dx);
  const bend = -Math.sin(z * 0.044) * 0.45 - Math.sin(z * 0.0175) * 0.55;
  const bendMag = clamp(Math.abs(bend), 0.0, 1.0);
  const wCh = 4.8 + 2.0 * Math.pow(Math.sin(z * 0.023 + 0.4), 2.0);
  const cutSide = (dx * (-bend)) / (Math.abs(dx) + 3.0);
  const bankScale = wCh + 3.2 - 1.6 * cutSide;
  const channelT = smoothstep(2.0, bankScale + 4.8, d);
  const bedY = -0.50 - 0.95 * bendMag + 0.12 * Math.sin(z * 0.19);
  const pointBar = Math.max(0.0, -cutSide) * Math.exp(-Math.pow((d - (wCh + 1.8)) / 3.4, 2.0)) * 0.95;

  const t1 = 1.95 + 0.55 * smoothstep(wCh + 2.0, wCh + 9.0, d);
  const t2 = t1 + 2.15 * smoothstep(24.0, 35.0, d);
  const floodMicro = 0.20 * Math.sin(x * 0.07 + z * 0.05) + 0.10 * Math.cos(x * 0.15 - z * 0.11);
  const floodplainY = t2 + floodMicro;

  const slopeD = clamp((d - 46.0) / 220.0, 0.0, 1.25);
  const woodlandBase = 21.0 * Math.pow(slopeD, 1.15);
  const woodlandUndulation = (2.5 * Math.sin(x * 0.021 + 0.4) * Math.cos(z * 0.019 - 0.3)
    + 1.3 * Math.sin(x * 0.047 - z * 0.039)
    + 0.55 * noise(x * 0.55, z * 0.55)) * smoothstep(32.0, 75.0, d);

  const nRise = smoothstep(-95.0, -280.0, z);
  const nRidgeProfile = 40.0 * Math.pow(nRise, 1.22) * (0.78 + 0.22 * Math.sin(x * 0.018 + 1.1) * Math.cos(x * 0.009));
  const nTerrace = nRidgeProfile + 1.9 * Math.sin(nRidgeProfile * 0.55) * nRise * smoothstep(45.0, 90.0, d);

  const eHeadlandX = smoothstep(115.0, 275.0, x);
  const eHeadlandZ = smoothstep(-140.0, 40.0, z) * (1.0 - 0.35 * smoothstep(200.0, 300.0, z));
  const eHeadland = 30.0 * eHeadlandX * eHeadlandZ * (0.82 + 0.18 * Math.cos(z * 0.025 - 0.5));
  const eBench = eHeadland + 1.5 * Math.sin(eHeadland * 0.6) * eHeadlandX;

  const gZLine = -55.0 + (x + 75.0) * 0.72;
  const gDist = Math.abs(z - gZLine);
  const gAlong = smoothstep(-68.0, -120.0, x) * (1.0 - smoothstep(-245.0, -295.0, x));
  const gCarve = 10.5 * gAlong * Math.exp(-Math.pow(gDist / 26.0, 2.0));
  const gFlank = 5.0 * gAlong * Math.exp(-Math.pow((gDist - 42.0) / 18.0, 2.0));

  const slopeY = woodlandBase + woodlandUndulation + nTerrace + eBench - gCarve + gFlank;
  const landY = floodplainY + Math.max(0.0, slopeY) * smoothstep(38.0, 72.0, d);
  return bedY + pointBar + (landY - bedY - pointBar) * channelT;
}

export function naturalHeightAt(x, z) {
  const r = Math.sqrt(x * x + z * z);
  const sOuter = smoothstep(68.0, 112.0, r);
  const c = coreHeightAt(x, z);
  if (sOuter <= 0.0) return c;
  const o = outerHeightAt(x, z);
  return c + (o - c) * sOuter;
}

export function terrainMesh(side = 120, span = 600) {
  const positions = [], indices = [];
  for (let j = 0; j <= side; j++) for (let i = 0; i <= side; i++) {
    const x = (i / side - 0.5) * span, z = (j / side - 0.5) * span;
    positions.push(x, heightAt(x, z), z);
  }
  for (let j = 0; j < side; j++) for (let i = 0; i < side; i++) {
    const a = j * (side + 1) + i, b = a + side + 1;
    indices.push(a, b, a + 1, a + 1, b, b + 1);
  }
  return { positions, indices };
}

// The campsite is a designed clearing in the shared authored heightfield.
export const campsiteClearing = { centre: [-22, 28], flatRadius: 8, blendRadius: 13 };
export const campsiteGroundY = naturalHeightAt(...campsiteClearing.centre);

export function heightAt(x, z) {
  const d = Math.hypot(x - campsiteClearing.centre[0], z - campsiteClearing.centre[1]);
  if (d <= campsiteClearing.flatRadius) return campsiteGroundY;
  const natural = naturalHeightAt(x, z);
  if (d >= campsiteClearing.blendRadius) return natural;
  const t = (d - campsiteClearing.flatRadius) / (campsiteClearing.blendRadius - campsiteClearing.flatRadius);
  const blend = t * t * (3.0 - 2.0 * t);
  return campsiteGroundY + (natural - campsiteGroundY) * blend;
}
