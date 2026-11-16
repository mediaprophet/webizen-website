"""Re-link the website adapter against an already-built QualiaDB library."""
import argparse,hashlib,json,subprocess
from pathlib import Path
HERE=Path(__file__).resolve().parent
DEFAULT=Path('C:/github/synth/target-prod/debug/deps')
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--dependency-dir',type=Path,default=DEFAULT);p.add_argument('--qualia-library',type=Path);p.add_argument('--serde-json-library',type=Path);a=p.parse_args();core=a.qualia_library or a.dependency_dir/'libqualia_core_db.rlib';serde=a.serde_json_library or next(a.dependency_dir.glob('libserde_json-*.rlib'));out=HERE/'q42_web.exe';src=HERE/'q42_web.rs'
 command=['rustc','--edition=2021',str(src),'--extern','qualia_core_db='+str(core),'--extern','serde_json='+str(serde),'-L','dependency='+str(a.dependency_dir),'-o',str(out)]
 subprocess.run(command,check=True)
 record={'schema':'webizen.native-reader-build/v1','compiler':subprocess.check_output(['rustc','--version'],text=True).strip(),'adapter_source_sha256':sha(src),'adapter_binary_sha256':sha(out),'qualia_library_sha256':sha(core),'serde_json_library_sha256':sha(serde),'reuses_existing_library':True,'kernel_rebuilt':False,'public_api':['UnifiedVolumeBuilder::with_lex_map','UnifiedVolumeBuilder::with_permissive_commons','Q42Volume::open','Q42Volume::verify_all_blocks','Q42Volume::read_all_quins','Q42LexMmap::lookup_owned','mini_parser::hash_token']}
 (HERE/'reader-build.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8');print(json.dumps(record))