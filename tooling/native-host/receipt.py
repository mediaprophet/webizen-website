from pathlib import Path
import argparse,hashlib,json,subprocess,shutil,tempfile,os,tomllib
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
p=argparse.ArgumentParser();p.add_argument('--output',type=Path,default=ROOT/'site/pkg/native-world');p.add_argument('--bindgen',type=Path,required=True);a=p.parse_args()
files=[]
for name in ['webizen_world.js','webizen_world_bg.wasm','webizen_world.d.ts','webizen_world_bg.wasm.d.ts']:
 f=a.output/name;files.append({'path':str(f),'bytes':f.stat().st_size,'sha256':sha(f)})
assert files[1]['bytes']<32*1024*1024
for source in (ROOT/'native/world-host/src').rglob('*.rs'):
 text=source.read_text(encoding='utf-8')
 assert '\ufffd' not in text, f'Replacement character in {source}; host source must be valid UTF-8 so every language can live in the served copy'
sources=[{'path':str(f.relative_to(ROOT)),'sha256':sha(f)} for f in sorted((ROOT/'native/world-host/src').rglob('*.rs'))]
def tree_fingerprint(entries):
 return hashlib.sha256("\n".join(f"{e['path']}:{e['sha256']}" for e in entries).encode()).hexdigest()
core=Path('C:/github/qualiaDB/crates/qualia-core-db')
core_sources=[{'path':str(f.relative_to(core)).replace('\\','/'),'sha256':sha(f)} for f in sorted((core/'src').rglob('*.rs'))]
core_sources.append({'path':'Cargo.toml','sha256':sha(core/'Cargo.toml')})
core_head=subprocess.check_output(['git','-C',str(core),'rev-parse','HEAD'],text=True).strip()
core_dirty=subprocess.check_output(['git','-C',str(core),'status','--porcelain','--untracked-files=no','--','.'],text=True).splitlines()
core_untracked=subprocess.check_output(['git','-C',str(core),'ls-files','--others','--exclude-standard','--','src','Cargo.toml'],text=True).splitlines()
metadata=json.loads(subprocess.check_output(['cargo','metadata','--offline','--locked','--format-version','1','--filter-platform','wasm32-unknown-unknown'],cwd=ROOT/'native/world-host'))
resolved=[{k:p[k] for k in ['name','version','source','manifest_path']} for p in metadata['packages'] if p['name'].startswith('etalon') or p['name'] in ['naga','naga-types']]
expected='c110aaf319798c1f547360995dd0461a5396c6f2'
etalon_root=Path('C:/github/etalon')
etalon_head=subprocess.check_output(['git','-C',str(etalon_root),'rev-parse','HEAD'],text=True).strip()
assert etalon_head==expected, f'Local etalon HEAD {etalon_head} must be {expected}'
# cargo metadata --offline --filter-platform wasm32-unknown-unknown: 24 etalon/naga
# provider crates. Git pin: source ends in #expected. Path patches: source is
# null and manifest_path is under C:/github/etalon at that HEAD. Cargo.lock
# git providers are 31 when unpatched; 0 when every etalon/naga package is a
# path patch (source line dropped). Counts measured 2026-11-17; do not invent.
assert len(resolved)==24, f'expected 24 wasm etalon/naga providers, got {len(resolved)}'
def is_local_etalon(p):
 mp=p['manifest_path'].replace('\\','/')
 return 'C:/github/etalon' in mp or mp.lower().startswith(str(etalon_root).replace('\\','/').lower())
git_pinned=all(p.get('source') and str(p['source']).endswith('#'+expected) for p in resolved)
path_patched=all((not p.get('source')) and is_local_etalon(p) for p in resolved)
assert git_pinned or path_patched, 'Renderer family must resolve to git pin or local C:/github/etalon path patches at that pin'
(HERE/'resolved-etalon-crates.json').write_text(json.dumps(resolved,indent=2),encoding='utf8')
lock_providers=[p for p in tomllib.loads((ROOT/'native/world-host/Cargo.lock').read_text())['package'] if 'etalon.git' in p.get('source','')]
if git_pinned:
 assert len(lock_providers)==31 and all(p['source'].endswith('#'+expected) for p in lock_providers)
 source_kind='pushed immutable Git revision; no local provider overrides'
else:
 source_kind='local path patches of C:/github/etalon at git revision '+expected
render_crate=Path(next(p['manifest_path'] for p in resolved if p['name']=='etalon-render')).parent
render_provider={'repository':'https://github.com/mediaprophet/etalon.git','revision':expected,'source_kind':source_kind,'emf_shader_sha256':sha(render_crate/'src/shaders/emf_volumetric.wgsl'),'emf_cpu_reference_sha256':sha(render_crate/'src/emf.rs'),'resolved_crates':resolved,'resolved_wasm_provider_crates':len(resolved),'locked_all_platform_provider_crates':len(lock_providers)}

identity={'schema':'webizen.native-runtime-identity/v1','identity_kind':'served build identity; browser QA must compare served WASM bytes with this digest','wasm':{'url':'/pkg/native-world/webizen_world_bg.wasm','bytes':files[1]['bytes'],'sha256':files[1]['sha256']},'generated_loader_sha256':files[0]['sha256'],'host_source_fingerprint':tree_fingerprint(sources),'cargo_lock_sha256':sha(ROOT/'native/world-host/Cargo.lock'),'render_provider':render_provider,'native_core':{'repository':'qualiaDB','revision':core_head,'tracked_dirty':core_dirty,'untracked_source_files':core_untracked,'source_fingerprint':tree_fingerprint(core_sources),'source_file_count':len(core_sources)},'compiler':subprocess.check_output(['rustc','--version'],text=True).strip()}
(a.output/'runtime-identity.json').write_text(json.dumps(identity,indent=2),encoding='utf-8')
result={'schema':'webizen.native-world-build/v1','compiler':subprocess.check_output(['rustc','--version'],text=True).strip(),'binding_generator':subprocess.check_output([str(a.bindgen),'--version'],text=True).strip(),'binding_generator_sha256':sha(a.bindgen),'source_files':sources,'cargo_lock_sha256':sha(ROOT/'native/world-host/Cargo.lock'),'native_renderer':'QualiaPortal / PortalGpu / Etalon','controller':'Rust/WASM','vibe_runtime':'PoetSnapshot::eval_fn','q42_reader':'WasmSliceQ42Reader','authored_javascript_world_logic':False,'outputs':files,'runtime_visual_acceptance':'pending browser QA','runtime_identity':identity}
bundle_key=hashlib.sha256(json.dumps(identity,sort_keys=True).encode()).hexdigest()
qualified=a.output/'qualified'
qualified.mkdir(parents=True,exist_ok=True)
bundle=qualified/bundle_key
if not bundle.exists():
 with tempfile.TemporaryDirectory(prefix='.preparing-',dir=qualified) as scratch:
  prepared=Path(scratch)
  for name in ['webizen_world.js','webizen_world_bg.wasm','webizen_world.d.ts','webizen_world_bg.wasm.d.ts','runtime-identity.json']:
   shutil.copy2(a.output/name,prepared/name)
  assert sha(prepared/'webizen_world_bg.wasm')==files[1]['sha256']
  assert sha(prepared/'webizen_world.js')==files[0]['sha256']
  os.rename(prepared,bundle)
result['qualified_bundle']={'path':str(bundle),'identity_sha256':bundle_key,'publication':'complete directory renamed into place after checksum verification'}
(HERE/'build-receipt.json').write_text(json.dumps(result,indent=2),encoding='utf-8');print(json.dumps({'wasm_bytes':files[1]['bytes'],'wasm_sha256':files[1]['sha256'],'generated_loader':files[0]['path'],'qualified_bundle':str(bundle)}))
