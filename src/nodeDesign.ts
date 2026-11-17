export type Role='main'|'skill'|'subflow'|'document'|'script'|'file'|'missing';
export type Item={id:string;label:string;kind:string;title:string;summary:string;headings:string[]};
export type Link={source:string;target:string;line:number;missing:boolean};
export const designs:Record<Role,{label:string;color:string;icon:string;description:string}>={
 main:{label:'主 Skill',color:'#b69aff',icon:'◈',description:'独立技能入口 · 编排与任务分发'},
 skill:{label:'关联 Skill',color:'#818cf8',icon:'⬡',description:'被其他 Skill 引用的独立技能入口'},
 subflow:{label:'内部子流程',color:'#38bdf8',icon:'⇢',description:'执行协议 / 委派流程 · 非独立注册 Skill'},
 document:{label:'参考文档',color:'#94a3b8',icon:'▤',description:'规范、知识、模板与辅助资料'},
 script:{label:'执行脚本',color:'#34d399',icon:'⌘',description:'本地脚本 · 此应用不会执行'},
 file:{label:'资源文件',color:'#cbd5e1',icon:'◇',description:'被引用的本地资源'},
 missing:{label:'失效引用',color:'#fb7185',icon:'!',description:'目标文件不存在或无法解析'},
};
export function buildIndex(items:Item[],edges:Link[]){
 const byId=new Map(items.map(n=>[n.id,n]));
 const incoming=new Map<string,Set<string>>(),outgoing=new Map<string,Set<string>>();
 for(const e of edges){if(!incoming.has(e.target))incoming.set(e.target,new Set());incoming.get(e.target)!.add(e.source);if(!outgoing.has(e.source))outgoing.set(e.source,new Set());outgoing.get(e.source)!.add(e.target);}
 const roles=new Map<string,Role>();
 for(const n of items){
  const parents=[...(incoming.get(n.id)??[])];
  let role:Role=n.kind==='skill'?(parents.some(id=>byId.get(id)?.kind==='skill')?'skill':'main'):n.kind==='script'?'script':n.kind==='missing'?'missing':n.kind==='document'?'document':'file';
  // Conservative hint only: a directly referenced document with a workflow-like
  // TITLE, not just a ubiquitous "steps" heading somewhere in its contents.
  if(role==='document'&&parents.some(id=>byId.get(id)?.kind==='skill')&&/流程|工作流|编写|构建|验收|authoring|workflow|builder|acceptance/i.test(n.title))role='subflow';
  roles.set(n.id,role);
 }
 return {incoming,outgoing,roles};
}
