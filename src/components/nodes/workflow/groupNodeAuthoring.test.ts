import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { parse, compile } from 'svelte/compiler';

const source=readFileSync(new URL('./NodeGroupNode.svelte',import.meta.url),'utf8');
const editor=readFileSync(new URL('./GroupJsonFilterPathEditor.svelte',import.meta.url),'utf8');
const tree=parse(editor,{modern:true});
function findAttribute(value: unknown,name:string): Record<string, unknown> | undefined {
  if (!value || typeof value!=='object') return undefined;
  const object=value as Record<string,unknown>;
  if(object.type==='Attribute' && object.name===name) return object;
  for(const child of Object.values(object)) {
    if(Array.isArray(child)){for(const item of child){const found=findAttribute(item,name);if(found)return found;}}
    else {const found=findAttribute(child,name);if(found)return found;}
  }
  return undefined;
}

test('actual group card callback forwards its exact group/internal identity and saved expectations',async()=>{
  const attribute=findAttribute(tree.fragment,'applyNodeData')!;
  const value=attribute.value as {expression:{start:number;end:number}};
  const callback=editor.slice(value.expression.start,value.expression.end);
  const saved={path:'old',opaque:{keep:true}};
  const calls:unknown[]=[];
  const context=vm.createContext({groupId:'group-A',ownerSessionId:'opening-session',node:{id:'inner',node_type:'json-filter',data:saved},
    updateGroupNodeData:async(...args:unknown[])=>{calls.push(args);return {status:'applied'};}});
  const apply=vm.runInContext(callback,context);
  const patch={path:'new'};await apply('inner',patch);
  assert.deepEqual(calls,[['group-A','inner','json-filter',saved,patch,'opening-session']]);
  assert.equal((calls[0] as unknown[])[3],saved);
});

test('registered group card and shared form compile without adding inner graph handles',()=>{
  assert.match(source, /\$isEditing && !\$isReadOnly/);
  assert.match(source, /JSON.stringify\(\[\$currentSessionId, id, node.id, node.node_type\]\)/);
  assert.match(source,/node.node_type === 'json-filter'/);
  const form=readFileSync(new URL('./JsonFilterPathEditor.svelte',import.meta.url),'utf8');
  assert.doesNotMatch(form, /<Handle|<BaseNode/);
  assert.doesNotThrow(()=>compile(source,{filename:'NodeGroupNode.svelte',generate:'client'}));
  assert.doesNotThrow(()=>compile(form,{filename:'JsonFilterPathEditor.svelte',generate:'client'}));
  assert.doesNotThrow(()=>compile(editor,{filename:'GroupJsonFilterPathEditor.svelte',generate:'client'}));
  assert.match(editor, /const ownerSessionId = untrack/);
  const registration=readFileSync(new URL('../../workflowGraphTypes.ts',import.meta.url),'utf8');
  assert.match(registration,/'node-group': NodeGroupNode/);
});

test('absent data maps to the published null default and every present non-object expectation stays exact',async()=>{
  const attribute=findAttribute(tree.fragment,'applyNodeData')!;
  const value=attribute.value as {expression:{start:number;end:number}};
  const callback=editor.slice(value.expression.start,value.expression.end);
  for(const saved of [undefined,null,42,[1,2]]) {
    let expected:unknown='unset';
    const context=vm.createContext({groupId:'group',ownerSessionId:'owner',node:{node_type:'json-filter',data:saved},
      updateGroupNodeData:async(_group:unknown,_node:unknown,_type:unknown,data:unknown)=>{expected=data;return {status:'applied'};}});
    await vm.runInContext(callback,context)('inner',{path:''});
    assert.deepEqual(expected,saved===undefined ? null : saved);
  }
  assert.match(editor,/data=\{invalidData \? \{\} : node.data\} \{invalidData\}/);
});
