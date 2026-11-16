from pathlib import Path
root=Path(__file__).resolve().parents[1]
p=root/'site/world/app.js';s=p.read_text(encoding='utf-8').replace("fetch('/data/journey.json')","fetch('/publication/journey.json')");p.write_text(s,encoding='utf-8')
p=root/'site/lab/lab.js';s=p.read_text(encoding='utf-8')
s=s.replace("if(data.readable){for(const x of data.readable)out.append(el('p',null,`${x.subject} → ${x.predicate} → ${x.object}`));}","""if(data.readable){const publication=await fetch('/publication/entities.json').then(r=>r.json());const labels=new Map(publication.entities.map(e=>[e.uri,e.title]));const label=uri=>labels.get(uri)||decodeURIComponent(uri.split('/').pop()).replaceAll('-',' ');for(const x of data.readable){const row=el('p');for(const[key,uri]of [['subject',x.subject],['object',x.object]]){if(key==='object')row.append(document.createTextNode(' · '+label(x.predicate)+' · '));const a=el('a',null,label(uri));a.href=uri.startsWith('https://webizen.au/world/')?'/#'+uri.split('/').pop():uri;row.append(a);}out.append(row);}}""")
p.write_text(s,encoding='utf-8')
p=root/'site/world/app.js';s=p.read_text(encoding='utf-8').replace("history.replaceState(null,'','/');","history.replaceState(null,'','/'+location.search);");p.write_text(s,encoding='utf-8')
print('World now reads native Q42 journey projection; Lab graph results use human-readable titles.')
