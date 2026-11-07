import json, sys
ne = sys.argv[1]; out = sys.argv[2]
land = json.load(open(ne + '/ne_110m_land.geojson'))
polys = []
for f in land['features']:
    g = f['geometry']
    for p in (g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]):
        ring = p[0]
        xs = [c[0] for c in ring]; ys = [c[1] for c in ring]
        polys.append((ring, min(xs), max(xs), min(ys), max(ys)))
def inside(x, y):
    for ring, x0, x1, y0, y1 in polys:
        if not (x0 <= x <= x1 and y0 <= y <= y1): continue
        c = False; n = len(ring); j = n - 1
        for i in range(n):
            xi, yi = ring[i]; xj, yj = ring[j]
            if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi: c = not c
            j = i
        if c: return True
    return False
STEP = 2.0; LAT0, LAT1 = 82.0, -56.0
cols = int(360 / STEP); rows = int((LAT0 - LAT1) / STEP)
parts = []; n = 0
for r in range(rows):
    lat = LAT0 - (r + 0.5) * STEP
    for c in range(cols):
        lon = -180 + (c + 0.5) * STEP
        if inside(lon, lat):
            parts.append(f"M{c} {r}h0"); n += 1
path = ''.join(parts)
cs = json.load(open(ne + '/ne_50m_admin_0_countries.geojson'))
cent = {}
for f in cs['features']:
    p = f['properties']
    cc = p.get('ISO_A2_EH') if p.get('ISO_A2') in (None, '-99') else p.get('ISO_A2')
    if not cc or cc == '-99': continue
    lx, ly = p.get('LABEL_X'), p.get('LABEL_Y')
    if lx is None: continue
    cent[cc.lower()] = [round(lx, 1), round(ly, 1)]
with open(out, 'w') as o:
    o.write('// GENERATED from Natural Earth (public domain, naturalearthdata.com): 110m land as a\n')
    o.write(f'// {STEP}° dot grid (equirectangular, lat {LAT0}…{LAT1}) and 50m label points per ISO country code.\n')
    o.write(f'export const GRID = {{ cols: {cols}, rows: {rows}, step: {STEP}, lat0: {LAT0} }} as const;\n')
    o.write(f'export const LAND = {json.dumps(path)};\n')
    o.write('/** Country label point, [lon, lat], by lowercase ISO 3166-1 alpha-2 code. */\n')
    o.write('export const CENTROIDS: Record<string, [number, number]> = ' + json.dumps(cent, separators=(',', ':')) + ';\n')
print(n, 'dots', len(cent), 'countries', cols, rows)
