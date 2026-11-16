from pathlib import Path
import json,subprocess,hashlib
r=Path('C:/github/synth/production/webizen-swarm/production/native-world'); c=json.loads((r/'cache/solar-array-three--panel.json').read_text()); exe=r/'provenance/attest.exe'; src=r/'inputs/solar-array-three--panel.glb';original=Path(c['native']['path']);d=r/'provenance/checks';d.mkdir(exist_ok=True)
for name in ['repeat-a.10d','repeat-b.10d']:
 subprocess.run([str(exe),str(original),str(src),str(d/name)],check=True,capture_output=True)
a=(d/'repeat-a.10d').read_bytes();b=(d/'repeat-b.10d').read_bytes();assert a==b
bad=bytearray(original.read_bytes());bad[-1]^=1;(d/'corrupt-input.10d').write_bytes(bad)
p=subprocess.run([str(exe),str(d/'corrupt-input.10d'),str(src),str(d/'should-not-exist.10d')],capture_output=True)
assert p.returncode!=0 and not (d/'should-not-exist.10d').exists()
report={'passed':True,'deterministic_reseal':True,'two_output_sha256':hashlib.sha256(a).hexdigest(),'corrupt_original_crc_rejected_before_output':True,'bad_crc_process_returncode':p.returncode}
(d/'result.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))
