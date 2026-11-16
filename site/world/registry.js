const environments=new Map(),exhibits=new Map();
export function registerEnvironment(definition){if(!definition?.id||!definition.build||environments.has(definition.id))throw Error('Invalid or duplicate environment');environments.set(definition.id,definition);}
export function registerExhibit(definition){if(!definition?.id||!Array.isArray(definition.position)||definition.position.length!==3||exhibits.has(definition.id))throw Error('Invalid or duplicate exhibit');exhibits.set(definition.id,Object.freeze(definition));}
export const getEnvironment=id=>environments.get(id);
export const getExhibit=id=>exhibits.get(id);
export const allExhibits=()=>[...exhibits.values()];
export function validateManifest(m){if(m.version!==1||!Array.isArray(m.exhibits)||m.exhibits.length>100)throw Error('Unsupported world manifest');for(const e of m.exhibits)registerExhibit(e);return m;}
