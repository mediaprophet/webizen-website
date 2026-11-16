"""Native Synth terrain surfacing; Python only packages retained inputs/outputs.

Habitat decisions execute in Vibe; raster, relief maps, import, material binding,
UV preservation and sealing execute through existing Hypermedia morphisms.
"""
from pathlib import Path
import collections, hashlib, json, shutil, struct, subprocess, sys, time
ROOT=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT.parent))
from native_bridge import SynthBridge
from build_art import lit
for folder in ['assets','inputs','recipes','evidence','previews']:(ROOT/folder).mkdir(exist_ok=True)
def save(path,value):path.write_text(json.dumps(value,indent=2),encoding='utf8')
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def pack_glb(part,maps):
    binary=bytearray();views=[];accessors=[]
    def blob(data,target=None):
        while len(binary)%4:binary.append(0)
        start=len(binary);binary.extend(data);record={'buffer':0,'byteOffset':start,'byteLength':len(data)}
        if target:record['target']=target
        views.append(record);return len(views)-1
    def attribute(values,fmt,kind,count,target):
        view=blob(struct.pack('<'+str(len(values))+fmt,*values),target)
        accessors.append({'bufferView':view,'componentType':5126 if fmt=='f' else 5125,'count':count,'type':kind})
        return len(accessors)-1
    p=part['positions'];indices=part['indices']
    position=attribute(p,'f','VEC3',len(p)//3,34962)
    accessors[position].update(min=[min(p[j::3]) for j in range(3)],max=[max(p[j::3]) for j in range(3)])
    # Exact coordinate-frame conversion to normalised planar UV; no remeshing.
    uv=[coordinate for i in range(0,len(p),3) for coordinate in [(p[i]+90)/180,(p[i+2]+90)/180]]
    texcoord=attribute(uv,'f','VEC2',len(p)//3,34962)
    index=attribute(indices,'I','SCALAR',len(indices),34963)
    images=[{'bufferView':blob(maps[name].read_bytes()),'mimeType':'image/png'} for name in ['colour','normal','orm']]
    transform={'KHR_texture_transform':{'scale':[30,30]}}
    material={'name':'Vibe habitat surface','pbrMetallicRoughness':{'baseColorFactor':[1,1,1,1],'baseColorTexture':{'index':0},'metallicFactor':0,'roughnessFactor':1,'metallicRoughnessTexture':{'index':2,'extensions':transform}},'normalTexture':{'index':1,'scale':0.45,'extensions':transform},'occlusionTexture':{'index':2,'extensions':transform},'doubleSided':True}
    doc={'asset':{'version':'2.0','generator':'Synth retained terrain with Vibe habitat atlas'},'extensionsUsed':['KHR_texture_transform'],'buffers':[{'byteLength':len(binary)}],'bufferViews':views,'accessors':accessors,'images':images,'samplers':[{'wrapS':10497,'wrapT':10497,'magFilter':9729,'minFilter':9987}],'textures':[{'source':i,'sampler':0} for i in range(3)],'materials':[material],'meshes':[{'primitives':[{'attributes':{'POSITION':position,'TEXCOORD_0':texcoord},'indices':index,'material':0}]}],'nodes':[{'mesh':0}],'scenes':[{'nodes':[0]}],'scene':0}
    encoded=json.dumps(doc,separators=(',',':')).encode();encoded+=b' '*(-len(encoded)%4);binary+=b'\0'*(-len(binary)%4)
    return struct.pack('<III',0x46546c67,2,28+len(encoded)+len(binary))+struct.pack('<II',len(encoded),0x4e4f534a)+encoded+struct.pack('<II',len(binary),0x004e4942)+binary
def main():
    bridge=SynthBridge(ROOT)
    try:
        size=64
        samples=[[-90+(x+.5)*180/size,-90+(z+.5)*180/size] for z in range(size) for x in range(size)]
        world=(ROOT.parents[1]/'site/world/living-world.vibe').read_text(encoding='utf-8-sig')
        field=(ROOT/'material-field.vibe').read_text()
        cache=ROOT/'evidence/field-evaluation.json';key=hashlib.sha256((world+field+lit(samples)).encode()).hexdigest()
        previous=json.loads(cache.read_text()) if cache.exists() else {}
        cells=[];batches=[]
        if previous.get('source_sha256')==key:
            batches=previous['native_batches'];cells=[v for batch in batches for v in batch['bindings']['cells']]
        else:
            for start in range(0,len(samples),32):
                evaluated=bridge.call('vibe_eval',{'source':world+'\n'+field+'\nlet cells=habitat_cells('+lit(samples[start:start+32])+');\nlet palette=habitat_palette();'})
                cells.extend(evaluated['bindings']['cells']);batches.append(evaluated)
        save(cache,{'source_sha256':key,'native_batches':batches,'sample_count':len(cells)})
        palette=batches[-1]['bindings']['palette'];paths=collections.defaultdict(list)
        for i,value in enumerate(cells):
            x=(i%size)*8;z=(i//size)*8
            paths[int(value)].append(f'M{x} {z}h8v8h-8Z')
        layers=[]
        for key,commands in sorted(paths.items()):
            for start in range(0,len(commands),700):layers.append({'path':''.join(commands[start:start+700]),'fill':[round(v*255) for v in palette[key]]})
        source='requires [capability("capability.invoke")];\n'
        source+='let habitat=capability.invoke("hypermedia.radiance.render_vector",'+lit({'width':512,'height':512,'layers':layers})+');\n'
        source+='let soft=capability.invoke("hypermedia.radiance.filter_rgba",{image:habitat,operation:"blur",sigma:3});\n'
        source+='let colour=capability.invoke("hypermedia.radiance.encode_rgba_png",{image:soft});\n'
        source+='let soil=capability.invoke("hypermedia.surface.synthesize_maps",{side:512,seed:619,tint:[0.5,0.5,0.5],roughness:0.97,metallic:0.0,relief:0.003,band_strength:0.0,contrast:0.23});\n'
        source+='let packed=capability.invoke("hypermedia.surface.pack_pbr_set",{colour:soil.colour,normal:soil.normal,orm:soil.orm});\n'
        source+='let mut asset={files:[{key:"habitat-colour.png",resource:colour},{key:"soil-normal.png",resource:packed.normal},{key:"soil-orm.png",resource:packed.orm}]};'
        (ROOT/'recipes/habitat.vibe').write_text(source)
        result=bridge.produce(source,'habitat');save(ROOT/'evidence/habitat-production.json',result)
        resources={r['key']:r for r in result['receipt']['resources']}
        maps={}
        for name,key in [('colour','habitat-colour.png'),('normal','soil-normal.png'),('orm','soil-orm.png')]:
            maps[name]=ROOT/'assets'/key;shutil.copyfile(resources[key]['path'],maps[name])
        original=ROOT.parent/'campsite/terrain/assets/catchment-clearing.mesh.json'
        part=json.loads(original.read_text())['parts'][0]
        glb=ROOT/'inputs/habitat-terrain.glb';glb.write_bytes(pack_glb(part,maps))
        source='''requires [capability("capability.invoke")];
let terrain=capability.invoke("hypermedia.manifold.import_mesh",{input:"terrain.glb",format:"glb"});
let native=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:terrain});
let mut asset={files:[{key:"habitat-terrain.10d",resource:native}]};'''
        (ROOT/'recipes/terrain.vibe').write_text(source)
        # This full terrain's native topology/replay exceeds the MCP session's
        # 180-second bound on this machine. Use the same producer's CLI path.
        run=ROOT/'artifacts'/('terrain-'+str(time.time_ns()))
        subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','produce','--quality-profile','production16','--source',str(ROOT/'recipes/terrain.vibe'),'--input','terrain.glb','glb',str(glb),'--out',str(run.with_suffix('.hmc')),'--export-dir',str(run)],check=True)
        subprocess.run([sys.executable,str(ROOT/'finalise.py'),str(run)],check=True)
    finally:bridge.close()
if __name__=='__main__':main()
