"""Read-only local review server; implementation stays in ecosystem WASM/Vibe."""
from pathlib import Path
from http.server import SimpleHTTPRequestHandler,ThreadingHTTPServer
from urllib.parse import urlsplit,unquote
ROOT=Path(__file__).resolve().parents[1]
class Review(SimpleHTTPRequestHandler):
    def translate_path(self,path):
        rel=unquote(urlsplit(path).path)
        if rel.startswith('/publication/'):
            base=ROOT/'publication/generated';rel=rel[len('/publication/'):]
        elif rel.startswith(('/production/','/data/','/evidence/','/plans/')):
            base=ROOT;rel=rel.lstrip('/')
        else:
            base=ROOT/'site';rel=rel.lstrip('/') or 'aaa-proof.html'
        target=(base/rel).resolve()
        return str(target if target.is_relative_to(base.resolve()) else base/'__refused__')
    def end_headers(self):
        for key,value in [('Cache-Control','no-store'),('Cross-Origin-Resource-Policy','same-origin'),('Cross-Origin-Opener-Policy','same-origin'),('Cross-Origin-Embedder-Policy','require-corp')]:self.send_header(key,value)
        super().end_headers()
if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port',type=int,default=8801)
    options=parser.parse_args()
    if not 1024<=options.port<=65535:parser.error('Use an unprivileged local port')
    ThreadingHTTPServer(('127.0.0.1',options.port),Review).serve_forever()
