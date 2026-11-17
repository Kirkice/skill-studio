import test from 'node:test';
import assert from 'node:assert/strict';
import {parseLayout} from '../src/layout.ts';

test('layout import validates and preserves positions and roles', () => {
 const value=parseLayout(JSON.stringify({version:1,positions:{'a/SKILL.md':{x:1,y:-2}},roles:{'a/SKILL.md':'skill'}}));
 assert.deepEqual({...value.positions}, {'a/SKILL.md':{x:1,y:-2}});
 assert.equal(value.roles['a/SKILL.md'], 'skill');
});
test('layout import rejects invalid versions, coordinates and roles', () => {
 for (const value of [null,{version:2,positions:{},roles:{}},{version:1,positions:{a:{x:'1',y:0}},roles:{}},{version:1,positions:{},roles:{a:'unknown'}}]) {
  assert.throws(()=>parseLayout(JSON.stringify(value)));
 }
});
