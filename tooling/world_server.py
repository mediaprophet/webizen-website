"""Local review host: fixed bounded demos only, no arbitrary source or file API."""
import argparse,json,hashlib,time,threading,sys
from pathlib import Path
from http.server import ThreadingHTTPServer,SimpleHTTPRequestHandler
from urllib.parse import urlsplit,unquote
sys.path.insert(0,str(Path(__file__).resolve().parent))
from http_range import satisfiable_range
from synth_bridge import SynthBridge
ROOT=Path(__file__).resolve().parents[1];DOCS=ROOT.parent/'qualiaDB'/'docs'
if not DOCS.exists():DOCS=Path('C:/github/qualiaDB/docs')
INTENTECO_ROOT=ROOT.parent/'eco-intent'
if not INTENTECO_ROOT.exists():INTENTECO_ROOT=Path('C:/github/eco-intent')
INTENTECO_SLUGS=frozenset({
 'eucalyptus-camaldulensis','eucalyptus-dalrympleana','eucalyptus-regnans','eucalyptus-melliodora',
 'banksia-integrifolia','acacia-pycnantha','dicksonia-antarctica',
 'poa-labillardierei','themeda-triandra','lomandra-longifolia','bursaria-spinosa',
 'wallabia-bicolor','phascolarctos-cinereus','vombatus-ursinus','macropus-giganteus',
 'ornithorhynchus-anatinus','dacelo-novaeguineae',
 'homo-sapiens',
})
def _resolve_intenteco(raw_path):
 """Resolve a read-only /intenteco/<slug>/<subpath> URL to an allowlisted eco-intent file."""
 if '\\' in raw_path:return None
 unq=unquote(raw_path)
 if '\\' in unq or not unq.startswith('/intenteco/'):return None
 rest=unq[11:]
 slug,sep,sub=rest.partition('/')
 if not sep or not sub or slug not in INTENTECO_SLUGS:return None
 parts=sub.split('/')
 if any(p in ('','.','..') for p in parts):return None
 base=(INTENTECO_ROOT/slug).resolve()
 dest=(base/sub).resolve()
 if not dest.is_relative_to(base) or not dest.is_file():return None
 return dest
sys.path.insert(0,str(ROOT/'adapters/qualiadb'))
from graph_paths import GraphPaths
public_graph=GraphPaths(ROOT/'publication/generated')
bridge=None;busy=threading.Semaphore(1)
def literal(value):
 if isinstance(value,dict):return '{'+','.join(k+':'+literal(v) for k,v in value.items())+'}'
 if isinstance(value,list):return '['+','.join(literal(v) for v in value)+']'
 return json.dumps(value,ensure_ascii=False)
def recipe(kind,value):
 header='requires [capability("capability.invoke")];\n'
 if kind=='terrain':
  args={'tile_resolution':64,'cell_spacing_m':1,'base_height_m':0,'width':512,'height':512,'strokes':[{'op':'raise','x':30,'y':32,'radius':25,'strength':value,'falloff':'gaussian'},{'op':'raise','x':46,'y':44,'radius':18,'strength':value*.7},{'op':'lower','x':18,'y':24,'radius':9,'strength':4}]}
  return header+f'let volume=capability.invoke("hypermedia.geodesy.sculpt_terrain",{literal(args)});\nlet png=capability.invoke("hypermedia.radiance.encode_rgba_png",{{image:volume.relief_rgba}});\nlet matter=capability.invoke("hypermedia.geodesy.compile_m64",{{heightfield:volume.heightfield}});\nlet mut asset={{files:[{{key:"terrain.png",resource:png}},{{key:"terrain.m64",resource:matter}}]}};'
 if kind=='light':
  args={'origin':[-2,-2,-2],'voxel_size':.5,'grid_dims':[1,1,1],'sources':[{'position':[0,0,0],'e_amplitude':[value,value/3,value/6],'wavelength_nm':550}]}
  return header+f'let volume=capability.invoke("hypermedia.electromagnetics.compile_emf",{literal(args)});\nlet image=capability.invoke("hypermedia.electromagnetics.render_spectral_slice",{{emf:volume,axis:"z",slice_index:4,width:512,height:512,exposure:0.15}});\nlet png=capability.invoke("hypermedia.radiance.encode_rgba_png",{{image:image}});\nlet mut asset={{files:[{{key:"field.emf",resource:volume}},{{key:"spectrum.png",resource:png}}]}};'
 if kind=='sound':
  return header+f'let tone=capability.invoke("hypermedia.acoustics.synthesize_tone",{{frequency:{value},sample_rate:24000,frames:24000,amplitude:0.15}});\nlet wav=capability.invoke("hypermedia.acoustics.encode_wav",{{audio:tone}});\nlet mut asset={{files:[{{key:"tone.wav",resource:wav}}]}};'
 raise ValueError('Unknown production recipe')
class Handler(SimpleHTTPRequestHandler):
 def translate_path(self,path):
  raw=urlsplit(path).path
  if raw.startswith('/intenteco/') or unquote(raw).startswith('/intenteco/'):
   dest=_resolve_intenteco(raw)
   return str(dest) if dest is not None else str(INTENTECO_ROOT/'__refused__')
  rel=unquote(raw)
  if rel.startswith('/source/'):base=DOCS;rel=rel[8:]
  elif rel.startswith('/publication/'):base=ROOT/'publication/generated';rel=rel[13:]
  elif rel.startswith(('/data/','/plans/','/evidence/','/artifacts/','/recipes/','/production/','/publication/','/migration/','/schemas/','/packages/')):base=ROOT;rel=rel.lstrip('/')
  else:base=ROOT/'site';rel=rel.lstrip('/') or 'index.html'
  dest=(base/rel).resolve()
  return str(dest if dest.is_relative_to(base.resolve()) else base/'__refused__')
 def end_headers(self):
  clean_path=unquote(urlsplit(self.path).path)
  is_cors_asset=clean_path.startswith(('/intenteco/','/production/terrain-v2/'))
  self.send_header('Cache-Control','no-store')
  self.send_header('Cross-Origin-Resource-Policy','cross-origin' if is_cors_asset else 'same-origin')
  self.send_header('Cross-Origin-Opener-Policy','same-origin')
  self.send_header('Cross-Origin-Embedder-Policy','require-corp')
  self.send_header('Accept-Ranges','bytes')
  if is_cors_asset:self.send_header('Access-Control-Allow-Origin','*')
  super().end_headers()
 def do_OPTIONS(self):
  if unquote(urlsplit(self.path).path).startswith(('/intenteco/','/production/terrain-v2/')):
   self.send_response(204);self.send_header('Access-Control-Allow-Methods','GET, HEAD, OPTIONS');self.send_header('Access-Control-Allow-Headers','Range');self.end_headers();return
  self.send_error(405)
 def json_response(self,data,status=200):
  raw=json.dumps(data,ensure_ascii=False).encode();self.send_response(status);self.send_header('Content-Type','application/json; charset=utf-8');self.send_header('Content-Length',str(len(raw)));self.end_headers();self.wfile.write(raw)
 def do_GET(self):
  if self.path=='/api/status':return self.json_response({'catalog_count':bridge.catalog.get('count'),'binary':'synth/target-prod/debug/synth.exe','execution':'local native MCP','arbitrary_source_api':False})
  rng=self.headers.get('Range')
  if rng and rng.startswith('bytes='):
   path=self.translate_path(urlsplit(self.path).path)
   try:data=Path(path).read_bytes()
   except OSError:return self.send_error(404)
   window=satisfiable_range(rng,len(data))
   if window is None:
    self.send_response(416);self.send_header('Content-Range',f'bytes */{len(data)}');self.end_headers();return
   start,end=window
   chunk=data[start:end+1]
   self.send_response(206)
   self.send_header('Content-Type','application/octet-stream')
   self.send_header('Content-Range',f'bytes {start}-{end}/{len(data)}')
   self.send_header('Content-Length',str(len(chunk)))
   self.end_headers();self.wfile.write(chunk);return
  super().do_GET()
 def do_POST(self):
  origin=self.headers.get('Origin');host=self.headers.get('Host')
  if origin and origin!='http://'+host:return self.json_response({'error':'Same-origin requests only'},403)
  if int(self.headers.get('Content-Length','0'))>4096:return self.json_response({'error':'Request too large'},413)
  if not busy.acquire(blocking=False):return self.json_response({'error':'A demonstration is already running. Try again after it finishes.'},409)
  try:
   body=json.loads(self.rfile.read(int(self.headers.get('Content-Length','0'))))
   kind=body.get('kind');value=body.get('value')
   if self.path!='/api/demo':return self.json_response({'error':'Unknown endpoint'},404)
   if kind in ('terrain','light','sound'):
    lo,hi={'terrain':(2,30),'light':(1,20),'sound':(110,880)}[kind]
    if not isinstance(value,(int,float)) or not lo<=value<=hi:raise ValueError('Parameter outside the supported range')
    source=recipe(kind,value);result=bridge.produce(source,kind);result['source']=source;return self.json_response(result)
   if kind=='query':
    if value not in ('all','information','knowledge','socioeconomic'):raise ValueError('Unknown public relationship')
    result=public_graph.relationships(value)
    return self.json_response({'query':'Native Q42 relationship selection: '+value,'result':{'returned':len(result['rows'])},'readable':result['rows'],'receipt':result,'execution':'Native QualiaDB Q42 reader','source_graph':result['graph_uri']})
   if kind=='rights':
    if value not in ('community_summary','individual_records'):raise ValueError('Unknown synthetic request')
    # Two authored norms. The actual QualiaDB evaluator determines whether each is active.
    norms=[{'subject':101,'predicate':17,'object':201,'context':301,'metadata':2000000000,'parity':0},{'subject':101,'predicate':18,'object':202,'context':301,'metadata':2000000000,'parity':0}]
    result=bridge.call('evaluate_modality',{'modality':'deontic','now_unix':1794787200,'quins':norms})
    index=0 if value=='community_summary' else 1;verdict=result['verdicts'][index]
    permitted=verdict.get('status')=='Active' and verdict.get('opcode')==17
    return self.json_response({'request':value,'permitted':permitted,'result':result,'norms':norms,'output':{'observation_count':12,'dataset':'synthetic community observations'} if permitted else None,'execution':'QualiaDB deontic evaluator via current Synth MCP; server applies selected active norm','scope':'Authored synthetic purpose-specific example, not general rights compliance or authentication'})
   if kind=='path':
    if value not in ('learn','research','create','participate'):raise ValueError('Unknown declared purpose')
    selected=public_graph.select(value,locale=body.get('locale','en-AU'),support_preferences=body.get('support_preferences',[]))
    payload=json.dumps({'purpose':value,'places':selected['places'],'graph_sha256':selected['selection_receipt']['graph_sha256']},separators=(',',':'))
    source='let mut asset={files:[{key:"path.json",kind:"json",text:'+json.dumps(payload)+'}]};'
    selected['production']=bridge.produce(source,'path');return self.json_response(selected)
   raise ValueError('Unknown demonstration')
  except (ValueError,KeyError) as e:self.json_response({'error':str(e)},400)
  except Exception as e:self.json_response({'error':str(e)},503)
  finally:busy.release()
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--port',type=int,default=8800);args=parser.parse_args()
 bridge=SynthBridge(ROOT)
 print('Synth operations:',bridge.catalog.get('count'),flush=True)
 pointer=json.loads((ROOT/'publication/generated/current.json').read_text(encoding='utf-8'))
 graph=ROOT/'publication/generated'/pointer['q42']
 if graph.exists():print('Graph:',bridge.call('ingest_ontology',{'path':str(graph),'context_hash':75001}),flush=True)
 server=ThreadingHTTPServer(('127.0.0.1',args.port),Handler)
 print(f'World preview http://127.0.0.1:{args.port}/',flush=True)
 try:server.serve_forever()
 finally:bridge.close()
