from pathlib import Path
from datetime import timedelta
import importlib.util,json,re,unittest
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('journey',ROOT/'tooling/journey/build.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)

def visit(a,b):
    # Synthetic coarse-cell centre, never a coordinate from the user's source.
    return {'startTime':a.isoformat(),'endTime':b.isoformat(),'visit':{'topCandidate':{'placeLocation':{'latLng':'-35,149'},'placeId':'PRIVATE_TEST_SENTINEL','semanticType':'HOME'}}}

class JourneyTests(unittest.TestCase):
    def test_cutoff(self):
        cut=module.START;end=module.END
        self.assertIsNone(module.clipped_visit(visit(cut-timedelta(days=10),cut)))
        self.assertIsNone(module.clipped_visit(visit(cut-timedelta(days=1),cut+timedelta(hours=5))))
        self.assertEqual(module.clipped_visit(visit(cut-timedelta(days=1),cut+timedelta(hours=6)))[0],cut)
        self.assertEqual(module.clipped_visit(visit(end-timedelta(hours=6),end+timedelta(days=2)))[1],end)
        self.assertIsNone(module.clipped_visit(visit(end,end+timedelta(days=1))))
    def test_data_minimisation(self):
        a=module.START
        data=module.export({'semanticSegments':[visit(a,a+timedelta(days=1))]})
        text=json.dumps(data)
        self.assertNotIn('PRIVATE_TEST_SENTINEL',text);self.assertNotIn('HOME',text)
        self.assertNotIn('-35',text);self.assertNotIn('149',text)
        self.assertEqual(data['chapters'][0]['month_bands'],['2025-11'])
    def test_month_crossing(self):
        a=module.parse_time('2026-01-31T20:00:00+11:00');b=a+timedelta(hours=12)
        self.assertEqual(module.month_bands(a,b),['2026-01','2026-02'])
    def test_export_allowlist_and_privacy(self):
        data=json.loads((ROOT/'data/journey.json').read_text(encoding='utf-8'))
        self.assertEqual(data['default_language'],'en-AU');self.assertEqual(len(data['chapters']),7)
        fields={'id','title','month_bands','relative_order','stay_band','artistic_palette','landscape_copy'}
        self.assertEqual([c['relative_order'] for c in data['chapters']],list(range(1,8)))
        for c in data['chapters']:
            self.assertEqual(set(c),fields)
            self.assertTrue(all('2025-11'<=m<='2026-11' and re.fullmatch(r'\d{4}-\d{2}',m) for m in c['month_bands']))
            self.assertTrue(all(re.fullmatch(r'#[0-9a-f]{6}',p) for p in c['artistic_palette']))
        public=[ROOT/'data/journey.json',*list((ROOT/'site/journey').rglob('*'))]
        for path in public:
            if not path.is_file():continue
            text=path.read_text(encoding='utf-8')
            self.assertNotRegex(text,r'(?i)latlng|latitude|longitude|placeid|semantictype|starttime|endtime|source_hash|source_path|C:[/\\]Users|Timeline\.json')
            self.assertNotRegex(text,r'\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}')
            self.assertNotRegex(text,r'-\d{2}(?:\.\d+)?\s*[,;]\s*1\d{2}(?:\.\d+)?')
    def test_chapter_links(self):
        text=(ROOT/'site/journey/index.html').read_text(encoding='utf-8')
        for c in json.loads((ROOT/'data/journey.json').read_text())['chapters']:
            self.assertIn('/?chapter='+c['id']+'#field-studio',text)
        self.assertIn('lang="en-AU"',text);self.assertIn('A visit record is not a campsite.',text)

if __name__=='__main__':unittest.main()
