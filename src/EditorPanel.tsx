import CodeMirror from '@uiw/react-codemirror';
import { markdown } from '@codemirror/lang-markdown';
import { designs, type Role } from './nodeDesign';
type Props = {selected:string; content:string; dirty:boolean; busy:boolean; conflict:boolean; role?:Role; onSave:()=>void; onReload:()=>void; onChange:(text:string)=>void; onRole:(role:string)=>void};
export function EditorPanel({selected,content,dirty,busy,conflict,role,onSave,onReload,onChange,onRole}:Props){
 return <section className="editor"><div className="panel-head"><div>文档编辑 {dirty&&<span className="modified">● 未保存</span>}</div><button className="primary" disabled={!selected||!dirty||busy} onClick={onSave}>保存</button></div><div className="editor-path">{selected||'选择节点开始编辑'}</div>{conflict&&<div className="conflict">文件已被外部修改或删除，当前编辑内容未覆盖。 <button disabled={busy} onClick={onReload}>重新加载</button></div>}{selected&&<div className="role-picker"><label>节点类型 <select value={role??'auto'} onChange={e=>onRole(e.target.value)}><option value="auto">自动识别</option>{Object.entries(designs).map(([key,d])=><option key={key} value={key}>{d.label}</option>)}</select></label><small>仅改变展示分类，点击「保存布局」保留</small></div>}{selected?<CodeMirror value={content} extensions={[markdown()]} theme="dark" onChange={onChange} height="100%"/>:<div className="editor-empty">点击图谱节点或左侧文件<br/>查看与编辑源文件</div>}</section>;
}
