"""Read-only source inventory; writes planning evidence beside this script.

Run: python docs/plans/webizen-site-renewal-2026-11-13/audit_site.py --date YYYY-MM-DD
Static references are candidates for review, not verified production failures.
No external dependencies, network requests, or application builds.
"""
from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import urlsplit, unquote
from collections import Counter
import csv
import hashlib
import json
import re
import argparse
from datetime import date

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--date', required=True, type=date.fromisoformat, help='Audit date in the user timezone')
AUDIT_DATE = parser.parse_args().date.isoformat()

OUT = Path(__file__).resolve().parent
DOCS = Path('C:/github/qualiaDB/docs')

class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.title = ''
        self.in_title = False
        self.refs = []
        self.ids = []
        self.h1 = 0
        self.description = False
        self.canonical = False
        self.lang = ''
    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == 'title': self.in_title = True
        if tag == 'html': self.lang = a.get('lang', '')
        if tag == 'h1': self.h1 += 1
        if a.get('id'): self.ids.append(a['id'])
        if tag == 'meta' and a.get('name') == 'description': self.description = True
        if tag == 'link' and a.get('rel') == 'canonical': self.canonical = True
        for key in ('href', 'src'):
            if a.get(key): self.refs.append((tag, key, a[key]))
    def handle_endtag(self, tag):
        if tag == 'title': self.in_title = False
    def handle_data(self, text):
        if self.in_title: self.title += text

def area(route):
    name = Path(route).name
    if name in ('emf-qac-showcase.html', 'emf-qac-architecture.html', 'audio-showcase.html'):
        return ('Fields and acoustic media', 'Integrate existing explanatory pages with typed profiles, real production receipts and clearly labelled illustrations')
    if name == 'hypermedia-studio.html':
        return ('Authoring and applications', 'Generate catalog and ownership from source; connect the existing studio overview to reviewed Synth works')
    if name == 'domains-solvers.html':
        return ('Worlds and science', 'Connect the existing domains and solver overview to validated workloads and human tasks')
    if route.startswith('playground/anatomy-studio/'):
        return ('Anatomy production previews', 'Retain source evidence; promote only visually reviewed assets into curated anatomy experience')
    if name == 'etalon-vibe-harness.html':
        return ('AI and compute', 'Retain device diagnostics with pinned source identity and explicit backend qualification')
    if route == 'playground/compare.html':
        return ('Knowledge and reasoning', 'Preserve RDF comparison with equal input and output validation')
    if route.startswith('_layouts/'): return ('Shared templates', 'Retain and rebuild shared shell')
    if route.startswith('webizen-desktop-UI-redo/') or '/init-draft-standards-wip-main/' in route:
        return ('Historical and design material', 'Preserve with draft/archive label; remove from default discovery')
    if name.startswith('progress-') or name.startswith('release-showcase-'):
        return ('Releases', 'Preserve dated evidence; replace current entry point with manifest-driven release hub')
    if 'bench' in route: return ('Benchmarks', 'Consolidate under evidence hub; preserve individual runnable workloads')
    if route.startswith('vibe/') or name in ('rdf-q42-vibe-demo.html', 'vibe-host-native-bridge.html'):
        return ('VibeScript', 'Refresh tooling/engine ownership, examples, execution profile and release evidence')
    if route.startswith('manuals/') or route.startswith('standards/'):
        return ('Documentation and standards', 'Version and index; distinguish normative, draft and implementation status')
    if name.startswith('qdnf') or route.startswith('bootstrap_gateway/'):
        return ('Networking and identifiers', 'Explain topology and authority; publish scoped qualification')
    if any(x in name for x in ('llm', 'forge', 'compute-engine', 'wasm-engine', 'zero-heap')):
        return ('AI and compute', 'Refresh backend/provider/runtime evidence; connect to Etalon and Anvil')
    if route.startswith('poet-app/') or any(x in name for x in ('design-studio', 'authoring', 'prompt-precision')):
        return ('Authoring and applications', 'Explain Poet/Synth/Hypermedia roles; add reproducible authoring journey')
    if any(x in name for x in ('agreement', 'values', 'cml', 'semantic-instruments', 'docuquin', 'escrow')):
        return ('Agency and cooperation', 'Build human-scale explanations and inspectable synthetic scenarios')
    if any(x in name for x in ('anatomy', 'living-world', 'science', 'scientific', 'spatial', 'graphics', 'universe')):
        return ('Worlds and science', 'Curate visuals, bounded controls, scientific scope and accessible alternatives')
    if any(x in name for x in ('graph', 'ontology', 'wordnet', 'sparql', 'data-formats', 'integrity', 'analytics', 'logic', 'modalities', 'semantics', 'neuro-symbolic')):
        return ('Knowledge and reasoning', 'Connect facts, source, query and human benefit; refresh supported profiles')
    if name in ('cost_model.html', 'monetization.html', 'approved_developers.html'):
        return ('Participation and economics', 'Rewrite terms, licensing and opportunity using approved current evidence')
    if route.startswith('tests/') or route.startswith('api-explorer/') or name in ('api.html', 'utils.html', 'read.html', 'phone-console.html', 'issues.html'):
        return ('Developer tools', 'Refresh and expose through developer hub; separate diagnostics from public onboarding')
    if name in ('index.html', 'advanced-features.html', 'release-matrix.html', 'network_webizen.html'):
        return ('Ecosystem entry points', 'Replace with audience journeys and shared module/capability data')
    return ('Utility and discovery', 'Review individually; assign canonical purpose and preserve needed routes')

pages = {}
for path in sorted(DOCS.rglob('*.html')):
    if OUT in path.parents: continue
    p = Page()
    raw = path.read_text(encoding='utf-8', errors='replace')
    p.feed(raw)
    pages[path] = (p, raw)

rows, missing, fragments, duplicates = [], [], [], []
for path, (p, raw) in pages.items():
    route = path.relative_to(DOCS).as_posix()
    section, action = area(route)
    dups = [x for x, n in Counter(p.ids).items() if n > 1]
    if dups: duplicates.append({'page': route, 'ids': dups})
    for tag, attribute, ref in p.refs:
        u = urlsplit(ref)
        if u.scheme or u.netloc or '{{' in ref or '{%' in ref: continue
        target_name = unquote(u.path)
        if target_name.startswith('/qualiaDB/'): target_name = target_name[len('/qualiaDB/'):]
        elif target_name.startswith('/'): target_name = target_name[1:]
        target = ((DOCS if u.path.startswith('/') else path.parent) / target_name).resolve() if target_name else path
        try: target.relative_to(DOCS)
        except ValueError: continue
        if target.is_dir(): target = target / 'index.html'
        exists = target.exists() or (target.suffix == '.html' and target.with_suffix('.md').exists())
        if not exists:
            missing.append({'page': route, 'reference': ref, 'tag': tag, 'attribute': attribute, 'resolved': target.relative_to(DOCS).as_posix()})
        elif u.fragment and target in pages and unquote(u.fragment) not in pages[target][0].ids:
            fragments.append({'page': route, 'reference': ref, 'note': 'May be a dynamic or named anchor; inspect rendered page'})
    rows.append({'source_route': route, 'title': p.title.strip(), 'section': section, 'proposed_action': action,
                 'review_state': 'static-inventory-only', 'h1_count': p.h1, 'lang': p.lang,
                 'has_source_meta_description': p.description, 'has_source_canonical': p.canonical,
                 'duplicate_id_count': len(dups), 'source_bytes': path.stat().st_size,
                 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})

with (OUT / 'page-inventory.csv').open('w', encoding='utf-8', newline='') as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0]))
    w.writeheader(); w.writerows(rows)
catalog = DOCS.parents[1] / 'hypermedia-production/crates/asset-production-vibe/src/operations_catalog.rs'
ops = re.findall(r'^\s*\("((?:hypermedia|qualia)\.[^"]+)"', catalog.read_text(encoding='utf-8'), re.M)
ontology_dir = catalog.parent / 'ontology'
ontology_ids = []
for source in sorted(ontology_dir.glob('morphisms_*.rs')):
    ontology_ids.extend(re.findall(r'^\s*id:\s*"((?:hypermedia|qualia)\.[^"]+)"', source.read_text(encoding='utf-8'), re.M))
report = {'scope': 'Local docs HTML source; no live, generated Jekyll, dynamic DOM or external-link verification',
          'date': AUDIT_DATE, 'html_files': len(rows), 'section_counts': dict(Counter(r['section'] for r in rows)),
          'source_without_meta_description': sum(not r['has_source_meta_description'] for r in rows),
          'source_without_canonical': sum(not r['has_source_canonical'] for r in rows),
          'missing_local_reference_candidates': missing, 'fragment_review_candidates': fragments,
          'duplicate_source_ids': duplicates, 'catalog_count': len(ops),
          'catalog_sha256': hashlib.sha256(catalog.read_bytes()).hexdigest(),
          'ontology_count': len(ontology_ids),
          'catalog_only_ids': sorted(set(ops) - set(ontology_ids)),
          'ontology_only_ids': sorted(set(ontology_ids) - set(ops)),
          'catalog_category_counts': dict(Counter(op.split('.')[1] for op in ops)),
          'catalog_ids': ops}
(OUT / 'source-audit.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
print(json.dumps({k: v for k, v in report.items() if k not in ('missing_local_reference_candidates', 'fragment_review_candidates', 'catalog_ids')}, indent=2))
print('Missing reference candidates:', len(missing), 'Fragment candidates:', len(fragments))
print('Sample missing local references:', json.dumps(missing[:18], indent=2))
