"""Read-only source inventory and hash-preserving website migration baseline.

No browser execution or benchmark measurement is inferred from this scanner.
Run with --check to compare the existing baseline to the current source tree.
"""
from __future__ import annotations
import argparse, collections, hashlib, json, re, subprocess
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[2]
DOCS = Path('C:/github/qualiaDB/docs')
REPOS = Path('C:/github')
WEBSITE = REPOS / 'webizen-website'
OUT = ROOT / 'migration/baseline'
EVIDENCE = ROOT / 'evidence/baseline'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

def encoded(data):
    return (json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True)+'\n').encode()

class Page(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True)
        self.ids=[]; self.refs=[]; self.inputs=[]; self.title=''; self.in_title=False
        self.feed(text)
    def handle_starttag(self, tag, attributes):
        a=dict(attributes)
        if a.get('id'): self.ids.append(a['id'])
        if tag=='title': self.in_title=True
        for key in ('src','href','poster'):
            if a.get(key): self.refs.append({'tag':tag,'attribute':key,'value':a[key]})
        if tag in ('input','select','textarea','button'):
            self.inputs.append({k:v for k,v in {'tag':tag,'id':a.get('id'),'type':a.get('type'),'name':a.get('name')}.items() if v})
    def handle_endtag(self, tag):
        if tag=='title': self.in_title=False
    def handle_data(self, text):
        if self.in_title: self.title+=text

def repo_state(name):
    root=REPOS/name
    record={'repository':name,'local_path':str(root),'exists':root.is_dir()}
    if not root.is_dir(): return record
    try:
        r=subprocess.run(['git','-C',str(root),'rev-parse','HEAD'],capture_output=True,text=True,timeout=8)
        record['commit']=r.stdout.strip() if r.returncode==0 else None
        if r.returncode: record['revision_error']=r.stderr.strip()[:400]
    except (OSError,subprocess.TimeoutExpired) as e:
        record['commit']=None; record['revision_error']=str(e)[:400]
    if not record.get('commit'):
        # The shell sandbox can deny git -C for a readable sibling checkout.
        # Reading HEAD/ref metadata is sufficient for the commit identity only.
        try:
            gitdir=root/'.git'
            if gitdir.is_file():
                pointer=gitdir.read_text().strip()
                if not pointer.startswith('gitdir: '):raise ValueError('Unknown git pointer')
                gitdir=(root/pointer[8:]).resolve()
            head=(gitdir/'HEAD').read_text().strip()
            if head.startswith('ref: '):
                ref=head[5:];path=gitdir/ref
                if path.is_file():head=path.read_text().strip()
                else:
                    lines=(gitdir/'packed-refs').read_text().splitlines()
                    head=next(line.split()[0] for line in lines if line.endswith(' '+ref))
            if re.fullmatch('[a-f0-9]{40}',head):
                record['commit']=head;record['identity_method']='read-only Git HEAD/ref metadata'
        except (OSError,ValueError,StopIteration):pass
    record['working_tree']='not assumed clean; individual captured source hashes identify observed bytes'
    return record

def route_kind(section, route):
    if section=='Benchmarks': return 'benchmark-entry'
    if route.startswith('tests/'): return 'test-entry'
    if section=='Historical and design material': return 'historical-document'
    if section=='Anatomy production previews': return 'generated-preview'
    if section=='Shared templates': return 'build-template'
    if section=='Releases': return 'release-document'
    return 'source-page'

def owner_hints(route):
    result=['qualiadb']
    rules=[('anatom','anatomy'),('vibe','vibescript'),('hypermedia','hypermedia'),('synth','synth'),('etalon','etalon'),('aether','aether'),('qdnf','qdnf'),('qbdl','qdnf'),('identifier','qdnf'),('cooper','cooperation')]
    for needle, module in rules:
        if needle in route and module not in result: result.append(module)
    return result

def local_ref(route, ref):
    value=ref['value']; parts=urlsplit(value)
    item=dict(ref)
    if parts.scheme or parts.netloc or value.startswith('//'):
        item['state']='external-or-inline'; return item
    if '{{' in value or '{%' in value:
        item['state']='build-template-reference'; return item
    path=unquote(parts.path)
    target=DOCS/path.lstrip('/') if path.startswith('/') else DOCS/Path(route).parent/path
    if not path: target=DOCS/route
    target=target.resolve()
    item['fragment']=parts.fragment or None
    if not target.is_relative_to(DOCS.resolve()):
        item['state']='outside-docs-root';return item
    item['resolved_source']=target.relative_to(DOCS).as_posix()
    if target.is_file():item['state']='source-present'
    elif target.is_dir():item['state']='directory-needs-built-index'
    else:item['state']='unresolved-source-candidate'
    return item

def build():
    previous=load(WEBSITE/'data/demos.json')['items']
    prior={r['source_route']:r for r in previous}
    files=sorted(DOCS.rglob('*.html'))
    pages=[]; unresolved=[]
    for path in files:
        route=path.relative_to(DOCS).as_posix(); old=prior.get(route,{})
        parsed=Page(path.read_text(encoding='utf-8',errors='replace'))
        section=old.get('section','Historical and design material' if route.startswith('plans/') else 'New source requiring classification')
        kind=route_kind(section,route)
        refs=[local_ref(route,r) for r in parsed.refs]
        for r in refs:
            if r['state']=='unresolved-source-candidate':unresolved.append({'route':route,**r})
        destination='/lab/experiment/'+old.get('id',route.removesuffix('.html').replace('/','.')) if kind=='benchmark-entry' else '/read/'+route.removesuffix('.html')
        if kind in ('historical-document','generated-preview','release-document'):destination='/archive/'+route
        if kind=='build-template':destination=None
        pages.append({'id':old.get('id',route.removesuffix('.html').replace('/','.')),'source_route':route,'title':parsed.title.strip() or old.get('title') or route,'section':section,'kind':kind,'source_repository':'qualiaDB','source_sha256':digest(path),'bytes':path.stat().st_size,'source_owner':'qualiaDB/docs','module_candidates':owner_hints(route),'owner_qualification':'source owner confirmed; semantic module attribution requires maintainer review','preserved_route':'/source/'+route,'proposed_destination':destination,'migration_state':'preserved-source; replacement-not-qualified','disposition':'replace shared template with site shell' if kind=='build-template' else ('preserve with archival context' if kind in ('historical-document','generated-preview','release-document') else 'recreate UI over qualified original workload and retain direct route until equivalent'),'evidence_state':'source-inspected; runtime-not-run','existing_controls':parsed.inputs,'fragments':sorted(set(parsed.ids)),'dependencies':refs,'prior_inventory_hash':old.get('source_sha256'),'changed_since_prior_inventory':old.get('source_sha256')!=digest(path),'new_since_prior_inventory':route not in prior})
    routes={r['source_route'] for r in pages}
    missing_prior=sorted(set(prior)-routes)
    registry=DOCS/'tests/suite-registry.js'; reg=registry.read_text(encoding='utf-8')
    suites=[]
    for name,relative in re.findall(r"import\s*\{\s*register\s+as\s+(\w+)\s*\}\s*from\s*['\"]([^'\"]+)",reg):
        path=(registry.parent/relative).resolve()
        rel=path.relative_to(DOCS).as_posix()
        group='both' if name=='regComparison' else 'native' if path.name.startswith('native-') else 'wasm' if path.name.startswith('wasm-') else 'logic'
        suites.append({'id':path.stem,'registration':name,'source_route':rel,'source_sha256':digest(path),'registration_mode':group,'execution_state':'not-run','reported_pass_count':None,'existing_route':'/source/tests/index.html','runner_source':'tests/test-runner.js','classification_basis':'static import and explicit current registry mode groups'})
    runners=sorted(set(DOCS.glob('js/*bench*.js'))|set(DOCS.glob('*bench*.js'))|{DOCS/'tests/test-runner.js',DOCS/'tests/runner.js',DOCS/'tests/run-headless.mjs',registry})
    runner_records=[{'source_route':p.relative_to(DOCS).as_posix(),'source_sha256':digest(p),'bytes':p.stat().st_size} for p in runners if p.is_file()]
    benchmark_files=sorted(DOCS.glob('*benchmark*.json'))
    snapshots=[]
    for p in benchmark_files:
        sha=digest(p);rel=f'benchmarks/{sha}/{p.name}';data=load(p)
        snapshots.append({'source_route':p.name,'source_sha256':sha,'preserved_file':rel,'bytes':p.stat().st_size,'schema_version':data.get('schema_version'),'source_timestamp':data.get('last_updated'),'evidence_state':'existing-recorded-artifact; not-rerun-or-independently-qualified','original_engine_labels':sorted(data.get('engines',{}))})
    modules=load(WEBSITE/'data/modules.json')['modules']
    modules.append({'id':'qdnf','name':'QDNF and identifier fabric','source_repository':'qualiaDB','purpose':'Identifier, naming and network fabric; public subsystem explicitly reconciled with the module atlas','publication_status':'source-mapped; runtime-not-qualified'})
    for m in modules:
        name=m.get('source_repository')
        m['owner_status']='repository-present; public claims require qualification' if name and (REPOS/name).is_dir() else 'requires-source-owner-reconciliation'
        m['local_repository']=str(REPOS/name) if name and (REPOS/name).is_dir() else None
        if m['id']=='webizen':
            m.update(source_repository='qualiaDB',local_repository=str(REPOS/'qualiaDB'),owner_status='implementation source mapped; product scope spans supporting repositories',ownership_evidence=['qualiaDB/README.md:93','qualiaDB/crates/webizen-studio/src/main.rs'])
        if m['id']=='poet':
            m['ownership_evidence']=['synth/README.md:6-7 states Poet is separate','qualiaDB/crates/qualia-mcp/src/tools/poet.rs is an adapter, not proof of application ownership']
        if m['id']=='qdnf':m['ownership_evidence']=['qualiaDB/docs/qdnf.html','webizen-website/plans/renewal/README.md identifies subsystem coverage']
    repository_names=sorted({m['source_repository'] for m in modules if m.get('source_repository')}|{'webizen-website'})
    repositories=[repo_state(name) for name in repository_names]
    route_records={'schema':'webizen.preservation-baseline/v1','default_locale':'en-AU','scope':'All HTML source routes, not a count of runnable demonstrations','source_repository':'qualiaDB','source_root':'docs','route_count':len(pages),'prior_route_count':len(prior),'missing_prior_routes':missing_prior,'routes':pages}
    docs_digest=hashlib.sha256(encoded([(p['source_route'],p['source_sha256']) for p in pages])).hexdigest()
    summary={'schema':'webizen.baseline-summary/v1','route_count':len(pages),'section_counts':dict(sorted(collections.Counter(p['section'] for p in pages).items())),'kind_counts':dict(sorted(collections.Counter(p['kind'] for p in pages).items())),'suite_registrations':len(suites),'suite_mode_counts':dict(sorted(collections.Counter(s['registration_mode'] for s in suites).items())),'benchmark_artifacts':len(snapshots),'runner_source_files':len(runner_records),'module_groups':len(modules),'source_route_digest_sha256':docs_digest,'runtime_execution_performed':False,'missing_prior_routes':missing_prior,'unresolved_source_reference_candidates':len(unresolved),'qualification':'counts describe source coverage; no runtime pass or benchmark rerun inferred'}
    bug_candidates={'schema':'webizen.baseline-triage/v1','qualification':'source-resolution candidates require generated-site/runtime confirmation','unresolved_source_references':unresolved,'known_semantic_risks':[{'source':'docs/js/benchmark-environment.js','issue':'Unprobed NPU reported false; review unknown-versus-absent representation','reproduce':'Inspect hardware metadata code and compare with actual device probe result','state':'source-identified; runtime-not-reproduced'},{'source':'data/modules.json','issue':'Poet application repository ownership unresolved','reproduce':'Compare separate-application statement in Synth README with module source ledger','state':'explicit unresolved owner'}]}
    outputs={OUT/'routes.json':route_records,OUT/'modules.json':{'schema':'webizen.module-source-baseline/v1','modules':modules},OUT/'tests.json':{'schema':'webizen.test-source-baseline/v1','registry_source_sha256':digest(registry),'suites':suites,'outcome_policy':'not-run is not pass; original runner retains skips and zero-assertion semantics'},OUT/'runners.json':{'schema':'webizen.runner-source-baseline/v1','runners':runner_records},OUT/'benchmarks.json':{'schema':'webizen.benchmark-preservation/v1','artifacts':snapshots},OUT/'repositories.json':{'schema':'webizen.repository-baseline/v1','repositories':repositories},OUT/'triage.json':bug_candidates,EVIDENCE/'summary.json':summary}
    copies={EVIDENCE/item['preserved_file']:DOCS/item['source_route'] for item in snapshots}
    assert len(routes)==len(pages) and len({p['id'] for p in pages})==len(pages), 'Duplicate route/ID'
    assert not missing_prior, f'Prior routes missing: {missing_prior}'
    assert all(p['disposition'] and p['source_owner'] for p in pages)
    return outputs,copies,summary

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    outputs,copies,summary=build();differences=[]
    for path,data in outputs.items():
        raw=encoded(data)
        if args.check:
            if not path.exists() or path.read_bytes()!=raw:differences.append(str(path))
        else:path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
    for dest,source in copies.items():
        raw=source.read_bytes()
        if dest.exists() and dest.read_bytes()!=raw:raise RuntimeError('Immutable preserved artifact mismatch: '+str(dest))
        if args.check:
            if not dest.exists():differences.append(str(dest))
        else:dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(raw)
    print(json.dumps(summary,indent=2))
    if differences:raise SystemExit('Baseline drift or missing artifact:\n'+'\n'.join(differences))
    print('PASS: baseline '+('matches source and preserved artifacts' if args.check else 'generated and validated'))

if __name__=='__main__':main()
