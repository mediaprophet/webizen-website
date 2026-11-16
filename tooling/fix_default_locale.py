from pathlib import Path
root=Path(__file__).resolve().parents[1]
p=root/'site/world/app.js';s=p.read_text(encoding='utf-8').replace('await setLocale(initialLocale());','await setLocale(initialLocale(),{persist:false});');p.write_text(s,encoding='utf-8')
print('Initial loading no longer records an automatic locale as an explicit language choice.')
