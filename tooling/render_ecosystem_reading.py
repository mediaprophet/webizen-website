"""Render the accessible reading companion from the exact qualified module data."""
from pathlib import Path
import argparse,hashlib,html,json
ROOT=Path(__file__).resolve().parents[1]
PUB=ROOT/'data/publications/aaa'; SOURCE=PUB/'modules/narratives.en-AU.json'
OUT=ROOT/'site/atlas/index.html'
H=lambda v:html.escape(str(v),quote=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))

def render():
 data=read(SOURCE); catalogue=read(PUB/'catalogue.json'); descriptor=next(d for d in catalogue['datasets'] if d['id']=='ecosystem-module-narratives-en-au')
 acceptance=read(PUB/'modules/native-validation.json')
 assert data['locale']=='en-AU' and len(data['modules'])==16
 assert descriptor['source']['narrative_sha256']==sha(SOURCE)
 assert descriptor['sha256']==sha(ROOT/descriptor['q42_url'].lstrip('/'))==acceptance['native_file_sha256']
 assert acceptance['passed'] and acceptance['all_rows_roundtrip']['lexicon_values_equal']
 assert descriptor['row_count']==acceptance['all_rows_roundtrip']['count']==402
 modules=data['modules']; names={m['id']:m['title'] for m in modules}; assert len(names)==16
 blocks=[]
 for i,m in enumerate(modules,1):
  working=m['native_demo_status']=='partial-native-proof'
  badge='Working examples in the lab' if working else 'Demonstration still to come'
  related=' '.join(f'<a href="#{H(r)}">{H(names[r])}</a>' for r in m['related_modules'])
  refs=[]
  for key in m['source_refs']:
   r=data['sources'][key]
   label=H(r['repository']+'/'+r['path'])
   linked=f'<a href="{H(r["reference_url"])}"><bdi>{label}</bdi></a>' if r.get('reference_url') else f'<bdi>{label}</bdi>'
   state='The inspected working bytes differ from the baseline revision, or are untracked; this includes possible line-ending differences.' if r['working_tree_modified_or_untracked'] else 'The inspected bytes match this repository revision.'
   refs.append(f'''<li>{linked}<dl class="source-meta"><dt>Revision</dt><dd><bdi><code>{H(r['revision'])}</code></bdi></dd><dt>Inspected SHA-256</dt><dd><bdi><code>{H(r['sha256'])}</code></bdi></dd></dl><p class="source-note">{state} {H(r['reference_scope'])}</p></li>''')
  blocks.append(f'''<article class="module" id="{H(m['id'])}" aria-labelledby="title-{H(m['id'])}">
 <div class="module-heading"><p class="number" aria-hidden="true">{i:02d}</p><h2 id="title-{H(m['id'])}">{H(m['title'])}</h2><span class="status{' working' if working else ''}">{badge}</span></div>
 <div class="module-content"><p class="what">{H(m['what'])}</p>
 <h3>Its particular role</h3><p>{H(m['distinctive'])}</p>
 <h3>Why it matters</h3><p>{H(m['importance'])}</p>
 <aside class="example" aria-label="Example for {H(m['title'])}"><h3>A use to picture</h3><p>{H(m['example'])}</p></aside>
 <h3>People, rights and agency</h3><p>{H(m['rights'])}</p>
 <p class="related"><strong>Works alongside</strong><br>{related}</p>
 <details class="evidence"><summary>What is demonstrated, and the source evidence</summary>
 <h3>Current demonstration scope</h3><p>{H(m['native_demo_scope'])}</p>
 <p>{H(m['application_owner_status'])}.</p>
 <h3>Inspected source records</h3><p>The references identify the repository baseline and the exact working bytes reviewed. A source link is evidence for this explanation, not a claim that its demonstration is complete.</p><ul class="source-list">{''.join(refs)}</ul>
 <p><a href="/data/publications/aaa/modules/narratives.en-AU.json" download>Download the complete explanation and source records</a></p></details>
 <a class="back-top" href="#module-index">Back to the module guide ↑</a></div></article>''')
 index=''.join(f'<li><a href="#{H(m["id"])}"><span>{H(m["title"])}</span></a></li>' for m in modules)
 return f'''<!doctype html>
<html lang="en-AU" dir="ltr">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="description" content="Understand the sixteen parts of Webizen’s human centric computing ecosystem: their roles, practical uses and relationship to people, rights and the living world."><meta name="color-scheme" content="light"><title>The ecosystem, explained — Webizen</title><link rel="stylesheet" href="/atlas/ecosystem.css"></head>
<body><a class="skip" href="#content">Skip to the explanations</a>
<header class="site-header wrap"><a class="brand" href="/">webizen<span>Human centric computing</span></a><nav aria-label="Main navigation"><a href="/aaa-proof.html">Explore the native world ↗</a><a href="#module-index">Module guide</a><a href="#reading-data">Data and sources</a></nav></header>
<main id="content" class="wrap" tabindex="-1"><section class="hero" aria-labelledby="page-title"><div><p class="eyebrow">The ecosystem · English, Australia</p><h1 id="page-title">What each part makes possible.</h1><p class="lead">A person’s knowledge, creative work, relationships and place in the living world belong together. These are the parts being developed to support them—and the different jobs each one does.</p></div><aside class="hero-aside" aria-label="About this guide"><strong>16 parts.<br>Connected purposes.</strong><p>Start with a name below, or read through the ecosystem. Each explanation includes a use to picture, its human significance and the evidence behind it.</p></aside></section>
<section class="native-invitation" aria-labelledby="try-title"><div><p class="eyebrow">Try a working subset</p><h2 id="try-title">Move from an explanation<br>to something you can inspect.</h2><p>The native world includes real graph queries, public rule fixtures, source-backed terrain and modelled field data. The module guide also covers work whose demonstrations are still to come.</p></div><div class="native-links"><a href="/aaa-proof.html">Open the native world and proof laboratory ↗</a><small>The examples use the actual ecosystem runtime. This reading page works without JavaScript.</small></div></section>
<nav class="index" id="module-index" aria-labelledby="guide-title"><div class="index-intro"><h2 id="guide-title">Find your starting point.</h2><p>Every name opens its full explanation on this page.</p></div><ol class="module-index">{index}</ol></nav>
{''.join(blocks)}
<section class="data-note" id="reading-data" aria-labelledby="data-title"><p class="eyebrow">The explanation is data, too</p><h2 id="data-title">Read it. Query it. Keep the source.</h2><p>These same sixteen explanations are stored in a native Q42 graph. In the proof laboratory’s Data and queries instrument, choose <strong>{H(descriptor['title'])}</strong> to ask about module purposes, relationships, rights or demonstration status. You can inspect the result and export the actual data.</p><div class="downloads"><a href="{H(descriptor['q42_url'])}" download>Download the native Q42 graph</a><a href="/data/publications/aaa/modules/narratives.en-AU.json" download>Download the readable JSON</a><a href="/data/publications/aaa/catalogue.json">Inspect the dataset catalogue</a><a href="/data/publications/aaa/modules/native-validation.json">Inspect native validation</a></div><details class="build-details"><summary>How this reading page is checked</summary><p>The page is generated from the source-reviewed Australian English narratives. Its source digest matches the dataset catalogue. The native builder verified every one of the {descriptor['row_count']} graph rows and nine query fixtures; this does not certify the outstanding module demonstrations as complete.</p><p>Native Q42 SHA-256: <bdi><code>{H(descriptor['sha256'])}</code></bdi><br>Readable source SHA-256: <bdi><code>{sha(SOURCE)}</code></bdi></p><p>{H(data['rights_interpretation'])}</p></details></section></main>
<footer class="site-footer wrap"><p>Webizen · A human centric computing ecosystem</p><p><a href="#page-title">Back to the beginning ↑</a> · <a href="/aaa-proof.html">Explore the native world</a></p></footer>
</body></html>
'''

if __name__=='__main__':
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args();text=render()
 if args.check:
  assert OUT.read_text(encoding='utf8')==text,'Reading page differs from the qualified source; regenerate it.'
  print('PASS: 16 complete explanations; source/catalogue/native-Q42 pins agree; generated page current.')
 else:
  snapshot=OUT.with_name('index.before-module-narratives.html')
  if OUT.exists() and not snapshot.exists():snapshot.write_bytes(OUT.read_bytes())
  OUT.write_text(text,encoding='utf8');print(str(OUT))
