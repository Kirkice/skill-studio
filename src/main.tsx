import React, { useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Position, type Node, type Edge } from '@xyflow/react';
import { GraphCanvas } from './GraphCanvas';
import { buildIndex, designs, type Item, type Role } from './nodeDesign';
import CodeMirror from '@uiw/react-codemirror';
import { markdown } from '@codemirror/lang-markdown';
import '@xyflow/react/dist/style.css';
import './style.css';

type Layout = Record<string,{x:number;y:number}>;
const layoutKey=(root:string)=>`skillsrider:layout:v1:${root}`;
type Graph = { root: string; nodes: Item[]; edges: {source:string;target:string;line:number;missing:boolean}[]; warnings:string[] };
function App(){
 const [graph,setGraph]=useState<Graph|null>(null), [selected,setSelected]=useState(''), [content,setContent]=useState(''), [original,setOriginal]=useState('');
 const [query,setQuery]=useState(''), [focus,setFocus]=useState(''), [status,setStatus]=useState('就绪'), [busy,setBusy]=useState(false), [creating,setCreating]=useState(false), [name,setName]=useState('');
 const [scope,setScope]=useState('direct'), [showMissing,setShowMissing]=useState(false);
 const [layout,setLayout]=useState<Layout>({}), [layoutDirty,setLayoutDirty]=useState(false);
 const [roleOverrides,setRoleOverrides]=useState<Record<string,Role>>({});
 const index=useMemo(()=>buildIndex(graph?.nodes??[],graph?.edges??[]),[graph]);
 const roleOf=(n:Item):Role=>roleOverrides[n.id]??index.roles.get(n.id)??'file';
 const dirty=content!==original;
 function loadLayout(root:string){try{const roles=JSON.parse(localStorage.getItem(`${layoutKey(root)}:roles`)??'{}');setRoleOverrides(Object.fromEntries(Object.entries(roles).filter(([,value])=>typeof value==='string'&&Object.hasOwn(designs,value))) as Record<string,Role>);}catch{setRoleOverrides({});}try{const raw=JSON.parse(localStorage.getItem(layoutKey(root))??'{}');const valid:Layout={};for(const [id,p] of Object.entries(raw)){const pos=p as {x:number;y:number};if(pos&&Number.isFinite(pos.x)&&Number.isFinite(pos.y))valid[id]=pos;}setLayout(valid);}catch{setLayout({});}setLayoutDirty(false);}
 function saveLayout(){if(!graph)return;try{localStorage.setItem(layoutKey(graph.root),JSON.stringify(layout));localStorage.setItem(`${layoutKey(graph.root)}:roles`,JSON.stringify(roleOverrides));setLayoutDirty(false);setStatus('布局已保存，下次打开该工作区自动恢复');}catch(e){setStatus(`布局保存失败：${e}`);}}
 const proceed=()=>!dirty||window.confirm('当前文件有未保存修改，是否放弃？');
 async function task(fn:()=>Promise<void>){setBusy(true);try{await fn();}catch(e){setStatus(String(e));}finally{setBusy(false);}}
 async function refresh(){const g=await invoke<Graph>('scan_workspace');setGraph(g);return g;}
 async function choose(){if(!proceed()||(layoutDirty&&!window.confirm('布局有未保存的拖动，是否放弃？')))return;await task(async()=>{const path=await open({directory:true,multiple:false});if(!path)return;const g=await invoke<Graph>('open_workspace',{path});setGraph(g);loadLayout(g.root);setSelected('');setContent('');setOriginal('');setFocus('');setStatus('工作区已打开');});}
 async function select(id:string){if(!proceed())return;await task(async()=>{const d=await invoke<{content:string}>('read_document',{path:id});setSelected(id);setContent(d.content);setOriginal(d.content);setStatus(id);});}
 async function save(){await task(async()=>{await invoke('save_document',{path:selected,content,original});setOriginal(content);await refresh();setStatus('已保存');});}
 const visible=useMemo(()=>{
  if(!graph)return [];
  let ids:Set<string>|null=null;
  if(focus){ids=new Set([focus]);for(const e of graph.edges){if(e.source===focus)ids.add(e.target);if(e.target===focus)ids.add(e.source);}}
  if(!focus&&scope!=='all'){
   ids=new Set(graph.nodes.filter(n=>n.kind==='skill').map(n=>n.id));
   if(scope==='direct'){
    const roots=new Set(ids);
    for(const e of graph.edges)if(roots.has(e.source))ids.add(e.target);
   }else{
    let changed=true;
    while(changed){changed=false;for(const e of graph.edges)if(ids.has(e.source)&&!ids.has(e.target)){ids.add(e.target);changed=true;}}
   }
  }
  return graph.nodes.filter(n=>(showMissing||n.kind!=='missing')&&(!ids||ids.has(n.id))&&(!query||`${n.id} ${n.title} ${n.summary}`.toLowerCase().includes(query.toLowerCase())));
 },[graph,focus,query,scope,showMissing]);
 const positions=useMemo(()=>{
  const ids=new Set(visible.map(n=>n.id)), depths=new Map<string,number>(), incoming=new Set<string>();
  const links=(graph?.edges??[]).filter(e=>ids.has(e.source)&&ids.has(e.target));
  links.forEach(e=>incoming.add(e.target));
  const queue=visible.filter(n=>!incoming.has(n.id)).map(n=>n.id);
  queue.forEach(id=>depths.set(id,0));
  const visit=()=>{for(let i=0;i<queue.length;i++){const source=queue[i];for(const edge of links){if(edge.source===source&&!depths.has(edge.target)){depths.set(edge.target,(depths.get(source)??0)+1);queue.push(edge.target);}}}};
  visit();
  for(const n of visible){if(!depths.has(n.id)){depths.set(n.id,0);queue.push(n.id);visit();}}
  const rows=new Map<number,number>();
  return new Map(visible.map(n=>{const col=depths.get(n.id)??0,row=rows.get(col)??0;rows.set(col,row+1);return [n.id,{x:col*440,y:row*370}];}));
 },[visible,graph]);
 const nodes:Node[]=useMemo(()=>visible.map(n=>({id:n.id,sourcePosition:Position.Right,targetPosition:Position.Left,position:layout[n.id]??positions.get(n.id)??{x:0,y:0},draggable:true,data:{label:<div className="rich-node"><span className="node-kind">{designs[roleOf(n)].icon} {designs[roleOf(n)].label}{roleOf(n)==='subflow'&&!roleOverrides[n.id]?' · 推定':''}</span><div className="node-purpose">{designs[roleOf(n)].description}</div><strong title={n.title||n.label}>{n.title||n.label}</strong><small title={n.id}>{n.id}</small>{n.summary&&<p className="node-summary" title={n.summary}>{n.summary}</p>}{n.headings.length>0&&<ul className="node-headings">{n.headings.slice(0,3).map((h,i)=><li key={i}>{h}</li>)}</ul>}<div className="node-stats"><span>引用 {index.outgoing.get(n.id)?.size??0}</span><span>被引用 {index.incoming.get(n.id)?.size??0}</span><span>{n.label}</span></div></div>},className:`role-${roleOf(n)}`,style:{borderColor:designs[roleOf(n)].color,background:selected===n.id?'#25304b':'#151e30',color:'#e2e8f0',width:roleOf(n)==='main'?360:320,borderRadius:roleOf(n)==='skill'?20:12}})),[visible,selected,positions,layout,index,roleOverrides]);
 const edges:Edge[]=useMemo(()=>{const ids=new Set(visible.map(n=>n.id));return (graph?.edges??[]).filter(e=>ids.has(e.source)&&ids.has(e.target)).map((e,i)=>({id:String(i),source:e.source,target:e.target,label:`L${e.line}`,animated:false,style:{stroke:e.missing?'#fb7185':'#64748b',strokeDasharray:e.missing?'5 4':undefined},data:{line:e.line}}));},[graph,visible]);
 return <div className="app"><header><div className="brand"><b>◈</b> SkillsRider <span>LOCAL WORKSPACE</span></div><div className="actions"><button onClick={()=>setCreating(true)} disabled={!graph||busy}>＋ 新建 Skill</button><button className="primary" disabled={busy} onClick={choose}>打开工作区</button></div></header>
 <div className="body"><aside><div className="section-title">EXPLORER <span>{graph?.nodes.filter(n=>n.kind==='skill').length??0} SKILLS</span></div><input placeholder="搜索文件或 Skill…" value={query} onChange={e=>setQuery(e.target.value)}/><div className="file-list">{visible.map(n=><button key={n.id} className={'file '+(selected===n.id?'active':'')} title={n.id} disabled={n.kind==='missing'||busy} onClick={()=>select(n.id)}><span style={{color:designs[roleOf(n)].color}} title={designs[roleOf(n)].label}>{designs[roleOf(n)].icon}</span><div>{n.title||n.label}<small>{n.id}</small></div></button>)}</div><div className="workspace">{graph?.root??'尚未打开工作区'}</div></aside>
 <main><div className="panel-head"><div>引用关系 <span>{visible.length} 节点 · {edges.length} 条引用</span></div><div className="graph-options"><button disabled={!graph||!layoutDirty} onClick={saveLayout}>{layoutDirty?'● 保存布局':'布局已保存'}</button><button disabled={!graph} onClick={()=>{setLayout({});setLayoutDirty(true);setStatus('已恢复自动布局，点击保存布局以保留');}}>重置布局</button><select aria-label="引用范围" value={scope} onChange={e=>{setScope(e.target.value);setFocus('');}}><option value="direct">Skill 直接引用</option><option value="recursive">Skill 递归引用</option><option value="all">全部文件</option></select><label><input type="checkbox" checked={showMissing} onChange={e=>setShowMissing(e.target.checked)}/>失效引用</label><button disabled={!selected} onClick={()=>setFocus(focus?'':selected)}>{focus?'显示全部':'聚焦当前'}</button><button disabled={!graph||busy} onClick={()=>task(async()=>{await refresh();setStatus('索引已刷新');})}>刷新</button></div></div><div className="canvas">{graph?<GraphCanvas key={`${graph.root}:${focus}:${query}:${scope}:${showMissing}`} nodes={nodes} edges={edges} onMove={(id,position)=>{setLayout(prev=>({...prev,[id]:position}));setLayoutDirty(true);}} onSelect={id=>{if(!id.startsWith('missing:'))void select(id);}} onReference={e=>{setStatus(`引用：${e.source} → ${e.target}（第 ${e.data?.line} 行）`);void select(e.source);}}/>:<div className="empty"><div>◈</div><h1>让 Skill 的关系清晰可见</h1><p>打开本地目录，探索 Skill、参考文档与脚本之间的引用。</p><button className="primary" onClick={choose}>选择 Skills 目录</button></div>}</div><div className="legend">{Object.entries(designs).map(([k,d])=><span key={k}><i style={{background:d.color}}/>{d.label}</span>)}<span>实线：明确链接 · 虚线：失效引用</span></div></main>
 <section className="editor"><div className="panel-head"><div>文档编辑 {dirty&&<span className="modified">● 未保存</span>}</div><button className="primary" disabled={!selected||!dirty||busy} onClick={save}>保存</button></div><div className="editor-path">{selected||'选择节点开始编辑'}</div>{selected&&<div className="role-picker"><label>节点类型 <select value={roleOverrides[selected]??'auto'} onChange={e=>{const value=e.target.value;setRoleOverrides(prev=>{const next={...prev};if(value==='auto')delete next[selected];else next[selected]=value as Role;return next;});setLayoutDirty(true);}}><option value="auto">自动识别</option>{Object.entries(designs).map(([role,d])=><option key={role} value={role}>{d.label}</option>)}</select></label><small>仅改变展示分类，点击「保存布局」保留</small></div>}{selected?<CodeMirror value={content} extensions={[markdown()]} theme="dark" onChange={setContent} height="100%"/>:<div className="editor-empty">点击图谱节点或左侧文件<br/>查看与编辑源文件</div>}</section></div>
 <footer><span className={busy?'working':''}>{busy?'处理中…':status}</span><span>{graph?.warnings.length?`${graph.warnings.length} 个扫描提示 · `:''}本地优先 · 不执行 Skill 脚本</span></footer>
 {creating&&<div className="overlay"><form onSubmit={e=>{e.preventDefault();void task(async()=>{await invoke('create_skill',{name});await refresh();setCreating(false);setName('');setStatus('Skill 已创建');});}}><h2>创建 Skill</h2><p>在当前工作区创建目录和 SKILL.md。</p><input autoFocus placeholder="my-new-skill" value={name} onChange={e=>setName(e.target.value)} pattern="[a-z0-9-]+" required maxLength={80}/><div className="actions"><button type="button" onClick={()=>setCreating(false)}>取消</button><button className="primary" disabled={busy}>创建</button></div><small>{status}</small></form></div>}
 </div>;
}
createRoot(document.getElementById('root')!).render(<App/>);
