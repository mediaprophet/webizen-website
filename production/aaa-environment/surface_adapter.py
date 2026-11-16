"""Copy retained native UV01 values into the lossless face exchange record.
Source layout: qualia-core-db/container_10d/{header,section,texture_coordinate_section}.rs.
No UV solving or geometry reconstruction happens here; Synth already authored/sealed them.
The consumer still verifies the native container and its receipt before rendering.
"""
import struct,math
from pathlib import Path

def retained_uvs(path,expected):
 b=Path(path).read_bytes();table,count=struct.unpack_from('<II',b,56);assert 0<count<=64 and table+count*24<=len(b)
 rows=[struct.unpack_from('<BBHIIIII',b,table+i*24) for i in range(count)]
 rows=[r for r in rows if r[0]==14];assert len(rows)==1
 typ,tier,res,off,size,stride,n,crc=rows[0];assert res==0 and stride==n==0 and off+size<=len(b)
 payload=b[off:off+size];magic,version,flags,n,record=struct.unpack_from('<4sHHII',payload)
 assert magic==b'UV01' and version==1 and flags==0 and n==expected and record==8 and size==16+n*8
 values=list(struct.unpack_from('<'+str(n*2)+'f',payload,16));assert all(math.isfinite(v) and abs(v)<=1e6 for v in values)
 return values
