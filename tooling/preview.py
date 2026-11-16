"""Read-only local review server for the separate website repo and sibling docs."""
import argparse
from pathlib import Path
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import unquote,urlsplit

REPO=Path(__file__).resolve().parents[1]
DOCS=REPO.parent/'qualiaDB'/'docs'
class Handler(SimpleHTTPRequestHandler):
 def translate_path(self,path):
  rel=unquote(urlsplit(path).path)
  if rel.startswith('/source/'):
   base=DOCS;rel=rel.removeprefix('/source/')
  elif rel.startswith(('/plans/','/data/','/evidence/','/schemas/')):
   base=REPO;rel=rel.lstrip('/')
  else:
   base=REPO/'site';rel=rel.lstrip('/') or 'index.html'
  target=(base/rel).resolve()
  if not target.is_relative_to(base.resolve()):return str(base/'__refused__')
  return str(target)
 def end_headers(self):
  self.send_header('Cache-Control','no-store')
  self.send_header('Cross-Origin-Resource-Policy','same-origin')
  self.send_header('Cross-Origin-Opener-Policy','same-origin')
  self.send_header('Cross-Origin-Embedder-Policy','require-corp')
  super().end_headers()

if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--port',type=int,default=8799);args=parser.parse_args()
 server=ThreadingHTTPServer(('127.0.0.1',args.port),Handler)
 print(f'Local review: http://127.0.0.1:{args.port}/',flush=True)
 server.serve_forever()
