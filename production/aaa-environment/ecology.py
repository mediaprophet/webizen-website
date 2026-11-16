"""Authored woodland understorey; forms are indicative, not a taxonomic reconstruction."""
import math,random
from build import loft,tube

def shrub(seed=53):
 rng=random.Random(seed);parts=[]
 for b in range(7):
  az=b*2.3999;height=.95+rng.random()*.55;reach=.40+rng.random()*.45
  path=[[math.cos(az)*reach*t**1.4,height*t,math.sin(az)*reach*t**1.4] for t in [0,.2,.4,.6,.8,1]]
  parts.append(loft('shrub-stem-'+str(b),tube(path,[.018,.016,.012,.009,.005,.001],7),'twig'))
  for s in range(5):
   t=.32+s*.13;base=[math.cos(az)*reach*t**1.4,height*t,math.sin(az)*reach*t**1.4];a=az+(-1 if s%2 else 1)*.9;length=.25+.18*rng.random()
   twig=[[base[0]+math.cos(a)*length*u,base[1]+.16*u,base[2]+math.sin(a)*length*u] for u in [0,.33,.66,1]]
   parts.append(loft(f'shrub-twig-{b}-{s}',tube(twig,[.004,.003,.002,.0005],5),'twig'))
   for n in range(5):
    for side in [-1,1]:
     u=.12+n*.17;p=[base[0]+math.cos(a)*length*u,base[1]+.16*u,base[2]+math.sin(a)*length*u];angle=a+side*1.05;leaflen=.10+.05*rng.random();rings=[]
     for q,w in [(0,.0004),(.2,.011),(.45,.017),(.75,.011),(1,.0003)]:
      x=p[0]+math.cos(angle)*leaflen*q;y=p[1]+leaflen*(.6*q-.8*q*q);z=p[2]+math.sin(angle)*leaflen*q;dx=-math.sin(angle)*w;dz=math.cos(angle)*w
      rings.append([[x-dx,y,z-dz],[x,y+.004*math.sin(q*math.pi),z],[x+dx,y,z+dz],[x,y-.0006,z]])
     p1=loft(f'leaf-{b}-{s}-{n}-{side}',rings,'shrub-leaf');p1['args'].update(interpolation='linear',samples=1);parts.append(p1)
 return parts

def litter(seed=219,count=38):
 rng=random.Random(seed);parts=[]
 for k in range(count):
  x=rng.uniform(-.8,.8);z=rng.uniform(-.6,.6);a=rng.uniform(0,math.tau);length=rng.uniform(.12,.26);width=rng.uniform(.012,.027);rings=[]
  for t in [0,.2,.45,.7,1]:
   w=width*math.sin(t*math.pi)+.0005;cx=x+math.cos(a)*length*t;cz=z+math.sin(a)*length*t;cy=.012+.035*math.sin(t*math.pi)+.012*t*t;dx=-math.sin(a)*w;dz=math.cos(a)*w
   rings.append([[cx-dx,cy,cz-dz],[cx,cy+.006,cz],[cx+dx,cy,cz+dz],[cx,cy-.001,cz]])
  p=loft('fallen-leaf-'+str(k),rings,'litter');p['args'].update(interpolation='linear',samples=1);parts.append(p)
 return parts
