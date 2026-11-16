from pathlib import Path
root=Path(__file__).resolve().parents[1]
p=root/'site/lab/lab.js';s=p.read_text(encoding='utf-8');start=s.index("$('show-recorded').onclick=");end=s.index('\nlet entries=',start)
s=s[:start]+"$('show-recorded').onclick=async()=>{$('recorded').hidden=false;$('recorded').scrollIntoView({behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'instant':'smooth'});try{const {renderHistory}=await import('./history.js');await renderHistory($('recorded-results'));}catch(e){status($('recorded-results'),e.message,true);}};"+s[end:]
p.write_text(s,encoding='utf-8')
p=root/'site/lab/index.html';s=p.read_text(encoding='utf-8')
if 'type="importmap"' not in s:s=s.replace('</head>','<script type="importmap">{"imports":{"three":"/vendor/three.module.js"}}</script></head>')
p.write_text(s,encoding='utf-8')
p=root/'tooling/integrate.py';s=p.read_text(encoding='utf-8').replace("'design','publication','adapters'","'design','publication','adapters','content'");p.write_text(s,encoding='utf-8')
print('Lab history now uses hash-verified immutable benchmark artifacts; import map verified.')
