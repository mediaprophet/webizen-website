from pathlib import Path
import hashlib,json,subprocess,sys,re
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).parent
CORE=Path('C:/github/qualiaDB/crates/qualia-core-db/src');DEPS=Path('C:/github/synth/target-prod/debug/deps')
mod=(CORE/'domains/geospatial/adapters/mod.rs').read_text(encoding='utf-8')
fetch=mod[mod.index('fn execute_http_request_bytes('):mod.index('/// Status-only variant')]
impl=mod[mod.index('impl DataAdapter for DemAdapter {'):mod.index('impl DataAdapter for OsmAdapter {')]
(HERE/'native_adapter_support.rs').write_text('// Exact native source excerpts from adapters/mod.rs, not replacement algorithms.\n'+fetch+impl,encoding='utf-8')
args=['rustc','--edition','2021',str(HERE/'main.rs'),'--extern','qualia_core_db='+str(DEPS/'libqualia_core_db.rlib'),'-L','dependency='+str(DEPS)]
for crate in ['serde_json','png','ciborium','reqwest','rustls','sha2']:
 libs=sorted(DEPS.glob('lib'+crate+'-*.rlib'));assert libs,crate
 args+=['--extern',crate+'='+str(libs[0])]
subprocess.run(args+['-o',str(HERE/'data-pack.exe')],check=True)
subprocess.run(args+['--test','-o',str(HERE/'data-pack-tests.exe')],check=True)
tests=subprocess.run([str(HERE/'data-pack-tests.exe'),'--nocapture'],capture_output=True);sys.stdout.buffer.write(tests.stdout);sys.stderr.buffer.write(tests.stderr)
e=ROOT/'evidence/aaa-data';e.mkdir(parents=True,exist_ok=True);(e/'native-tests.txt').write_bytes(tests.stdout+tests.stderr)
assert tests.returncode==0
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
sources=[HERE/'main.rs',HERE/'native_adapter_support.rs',CORE/'domains/geospatial/adapters/dem_adapter.rs',CORE/'domains/geospatial/adapters/mod.rs',CORE/'domains/geospatial/terrain_pipeline.rs',CORE/'domains/geospatial/dem.rs',CORE/'modalities/logic/deontic.rs',CORE/'query/mini_parser.rs']
receipt={'schema':'webizen.native-data-build/v1','passed':True,'native_tests':int(re.search(rb'(\d+) passed;',tests.stdout).group(1)),'compiler':subprocess.check_output(['rustc','--version']).decode().strip(),'source_binding':'Patched native DemAdapter source compiled directly using path module; private HTTP function and DataAdapter implementation exact source excerpts, not replacement algorithms; native terrain mesh and pipeline sources compiled directly; underlying codecs/query VM/deontic engine linked from cached native qualia_core_db.','native_library':{'path':str(DEPS/'libqualia_core_db.rlib'),'sha256':sha(DEPS/'libqualia_core_db.rlib')},'sources':[{'path':str(p),'sha256':sha(p)} for p in sources if p.exists()],'binary_sha256':sha(HERE/'data-pack.exe'),'test_binary_sha256':sha(HERE/'data-pack-tests.exe')}
(e/'native-build.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8')
