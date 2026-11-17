import test from 'node:test';
import assert from 'node:assert/strict';
import {buildIndex} from '../src/nodeDesign.ts';
const item=(id,kind,title='')=>({id,kind,title,label:id,summary:'',headings:[]});
const link=(source,target)=>({source,target,line:1,missing:false});
test('independent skills are distinct from Markdown subflows',()=>{
 const {roles}=buildIndex([item('a','skill'),item('b','skill'),item('flow','document','编写流程'),item('ref','document','安全规范')],[link('a','b'),link('a','flow'),link('a','ref')]);
 assert.equal(roles.get('a'),'main');assert.equal(roles.get('b'),'skill');assert.equal(roles.get('flow'),'subflow');assert.equal(roles.get('ref'),'document');
});
test('unreferenced workflow documents are not classified as child skills',()=>{
 assert.equal(buildIndex([item('flow','document','构建流程')],[]).roles.get('flow'),'document');
});
test('reference counts are unique and cycles terminate',()=>{
 const {incoming,outgoing}=buildIndex([item('a','document'),item('b','document')],[link('a','b'),link('a','b'),link('b','a')]);
 assert.equal(incoming.get('b').size,1);assert.equal(outgoing.get('a').size,1);
});
