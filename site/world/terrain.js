// One deterministic terrain definition shared with the Synth OBJ production input.
export function riverX(z){return Math.sin(z*.044)*10+Math.sin(z*.095)*3;}
export function noise(x,z){return Math.sin(x*.11+z*.07)*.5+Math.sin(x*.28-z*.16)*.22+Math.cos(z*.39+x*.2)*.12+Math.sin(x*.73+z*.61)*.05;}
export function naturalHeightAt(x,z){const d=Math.abs(x-riverX(z));const bank=1-Math.exp(-Math.pow(d/9,2));const valley=.35+bank*(2.4+Math.max(0,Math.abs(x)-15)*.22);const mountains=Math.pow(Math.max(0,(-z-25)/65),1.4)*(7+12*Math.pow(Math.sin(x*.055+1.2),2));return valley+mountains+noise(x,z)*bank*(1.4+mountains*.17)-Math.exp(-d*d/16)*1.4;}
export function terrainMesh(side=128){const positions=[],indices=[];for(let j=0;j<=side;j++)for(let i=0;i<=side;i++){const x=(i/side-.5)*180,z=(j/side-.5)*180;positions.push(x,heightAt(x,z),z);}for(let j=0;j<side;j++)for(let i=0;i<side;i++){const a=j*(side+1)+i,b=a+side+1;indices.push(a,b,a+1,a+1,b,b+1);}return{positions,indices};}

// The campsite is a designed clearing in the shared authored heightfield.
export const campsiteClearing={centre:[-22,28],flatRadius:8,blendRadius:13};
export const campsiteGroundY=naturalHeightAt(...campsiteClearing.centre);
export function heightAt(x,z){
 const d=Math.hypot(x-campsiteClearing.centre[0],z-campsiteClearing.centre[1]);
 if(d<=campsiteClearing.flatRadius)return campsiteGroundY;
 const natural=naturalHeightAt(x,z);
 if(d>=campsiteClearing.blendRadius)return natural;
 const t=(d-campsiteClearing.flatRadius)/(campsiteClearing.blendRadius-campsiteClearing.flatRadius);
 const blend=t*t*(3-2*t);
 return campsiteGroundY+(natural-campsiteGroundY)*blend;
}