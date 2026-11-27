//! Evidence-backed document-order sections and context token budgets.
use crate::core::Graph;
use pulldown_cmark::{Event,HeadingLevel,Parser,Tag,TagEnd};
use serde::Serialize;
use std::{collections::{HashMap,HashSet},path::Path};
#[derive(Clone,Serialize)]
pub struct Call{pub target:String,pub title:String,pub line:usize,#[serde(default)] pub tool:bool}
#[derive(Clone,Default,Serialize)]
pub struct Stage{
 pub title:String,pub line:usize,pub summary:String,pub calls:Vec<Call>,pub meaning:crate::meaning::Meaning,
 #[serde(default)] pub tokens:usize,#[serde(default)] pub code_tokens:usize,#[serde(default)] pub share:f32,#[serde(default)] pub explicit:bool,
}
#[derive(Clone,Default,Serialize)]
pub struct Flow{
 pub source:String,pub title:String,pub entry:bool,pub stages:Vec<Stage>,pub meaning:crate::meaning::Meaning,
 #[serde(default)] pub frontmatter_tokens:usize,#[serde(default)] pub body_tokens:usize,
 #[serde(default)] pub total_tokens:usize,#[serde(default)] pub ref_tokens:usize,#[serde(default)] pub ref_files:usize,
}
struct Heading{title:String,line:usize,start:usize,end:usize}
pub fn explicit(title:&str)->bool{
 let t=title.trim().to_lowercase();
 ["step ","phase ","阶段","步骤"].iter().any(|prefix|t.starts_with(prefix))
}
fn code_tokens_in(section:&str)->usize{
 let mut in_code=false;let mut buf=String::new();
 for event in Parser::new(section){match event{
  Event::Start(Tag::CodeBlock(_))=>in_code=true,
  Event::End(TagEnd::CodeBlock)=>{in_code=false;buf.push('\n');}
  Event::Text(t) if in_code=>buf.push_str(&t),
  Event::Code(t)=>{buf.push_str(&t);buf.push(' ');}
  _=>{}
 }}
 crate::core::estimate_tokens(&buf)
}
pub fn parse(source:&str,content:&str,title:&str,entry:bool,graph:&Graph)->Flow{
 let body=crate::preview::body(content);
 let frontmatter_len=content.len().saturating_sub(body.len());
 let frontmatter_tokens=crate::core::estimate_tokens(&content[..frontmatter_len]);
 let body_tokens=crate::core::estimate_tokens(&body);
 let total_tokens=frontmatter_tokens+body_tokens;
 let mut headings=vec![];let mut current:Option<Heading>=None;
 for(event,range)in Parser::new(content).into_offset_iter(){
  if range.start<frontmatter_len{continue;}
  match event{
   Event::Start(Tag::Heading{level:HeadingLevel::H2,..})=>current=Some(Heading{title:String::new(),line:content[..range.start].bytes().filter(|b|*b==b'\n').count()+1,start:range.start,end:range.end}),
   Event::Text(text)|Event::Code(text)=>if let Some(h)=&mut current{h.title.push_str(&text);},
   Event::End(TagEnd::Heading(HeadingLevel::H2))=>if let Some(mut h)=current.take(){h.end=range.end;headings.push(h);},
   _=>{}
  }
 }
 let denom=body_tokens.max(1) as f32;
 let mut stages=vec![];
 for(i,h)in headings.iter().enumerate(){
  let end=headings.get(i+1).map_or(content.len(),|next|next.start);
  let next_line=headings.get(i+1).map_or(usize::MAX,|next|next.line);
  let section_slice=&content[h.start.min(end)..end];
  let tokens=crate::core::estimate_tokens(section_slice);
  let code_tokens=code_tokens_in(section_slice).min(tokens);
  let share=((tokens as f32)/denom).clamp(0.,1.);
  let is_explicit=explicit(&h.title);
  let mut seen=HashSet::new();
  let calls=graph.edges.iter().filter(|e|e.source==source&&e.line>h.line&&e.line<next_line&&!e.missing).filter_map(|edge|{
   let node=graph.nodes.iter().find(|n|n.id==edge.target&&(matches!(n.kind.as_str(),"skill"|"document")||crate::core::is_tool(&n.kind)))?;
   if !seen.insert(node.id.clone()){return None;}
   Some(Call{target:node.id.clone(),title:if node.title.is_empty(){node.label.clone()}else{node.title.clone()},line:edge.line,tool:crate::core::is_tool(&node.kind)})
  }).collect();
  let mut paragraph=false;let mut summary=String::new();
  for event in Parser::new(&content[h.end.min(end)..end]){match event{
   Event::Start(Tag::Paragraph)=>paragraph=true,
   Event::End(TagEnd::Paragraph) if !summary.is_empty()=>break,
   Event::End(TagEnd::Paragraph)=>paragraph=false,
   Event::Text(t)|Event::Code(t) if paragraph=>summary.push_str(&t),
   Event::SoftBreak|Event::HardBreak if paragraph=>summary.push(' '),_=>{}
  }}
  let start=h.end.min(end);let base_line=content[..start].bytes().filter(|b|*b==b'\n').count()+1;
  let meaning=crate::meaning::extract(&h.title,h.line,&content[start..end],base_line);
  stages.push(Stage{title:h.title.clone(),line:h.line,summary:summary.chars().take(160).collect(),calls,meaning,tokens,code_tokens,share,explicit:is_explicit});
 }
 let mut extra=Vec::new();
 for edge in graph.edges.iter().filter(|e|e.source==source&&!e.missing){
  let Some(node)=graph.nodes.iter().find(|n|n.id==edge.target&&(crate::core::is_tool(&n.kind)||(headings.is_empty()&&matches!(n.kind.as_str(),"skill"|"document"))))else{continue;};
  if node.id==source||stages.iter().any(|s|s.calls.iter().any(|c|c.target==edge.target))||extra.iter().any(|c:&Call|c.target==edge.target){continue;}
  let title=if crate::core::is_tool(&node.kind)||node.title.is_empty(){node.label.clone()}else{node.title.clone()};
  extra.push(Call{target:node.id.clone(),title,line:edge.line,tool:crate::core::is_tool(&node.kind)});
 }
 if !extra.is_empty(){
  let intro_end=headings.first().map_or(content.len(),|h|h.start);
  let intro_slice=&content[frontmatter_len.min(intro_end)..intro_end];
  let tokens=crate::core::estimate_tokens(intro_slice);
  let code_tokens=code_tokens_in(intro_slice).min(tokens);
  stages.insert(0,Stage{title:"Referenced tools".into(),line:0,summary:String::new(),calls:extra,meaning:Default::default(),tokens,code_tokens,share:((tokens as f32)/denom).clamp(0.,1.),explicit:false});
 }
 let entry_end=headings.iter().find(|h|explicit(&h.title)).or_else(||headings.first()).map_or(content.len(),|h|h.start);
 Flow{source:source.into(),title:title.into(),entry,stages,meaning:crate::meaning::document(title,&content[..entry_end]),frontmatter_tokens,body_tokens,total_tokens,ref_tokens:0,ref_files:0}
}
pub fn collect(graph:&Graph)->Vec<Flow>{
 let mut flows:Vec<Flow>=graph.nodes.iter().filter(|n|matches!(n.kind.as_str(),"skill"|"document")).filter_map(|node|{
  let content=crate::core::read_document(Path::new(&graph.root),&node.id).ok()?.content;
  let title=if node.title.is_empty(){&node.label}else{&node.title};
  Some(parse(&node.id,&content,title,node.kind=="skill",graph))
 }).collect();
 let mut file_tokens:HashMap<String,usize>=flows.iter().map(|f|(f.source.clone(),f.total_tokens)).collect();
 for node in &graph.nodes{
  if crate::core::is_tool(&node.kind)&&!node.id.contains(':'){
   if let Ok(doc)=crate::core::read_document(Path::new(&graph.root),&node.id){
    file_tokens.insert(node.id.clone(),crate::core::estimate_tokens(&doc.content));
   }
  }
 }
 for i in 0..flows.len(){
  let root_id=flows[i].source.clone();
  let mut visited=HashSet::from([root_id.clone()]);
  let mut queue=vec![root_id];
  let mut ref_tokens=0usize;
  let mut ref_files=0usize;
  while let Some(curr)=queue.pop(){
   for edge in graph.edges.iter().filter(|e|e.source==curr&&!e.missing){
    if visited.insert(edge.target.clone()){
     if let Some(&tok)=file_tokens.get(&edge.target){
      ref_tokens+=tok;
      ref_files+=1;
     }
     queue.push(edge.target.clone());
    }
   }
  }
  flows[i].ref_tokens=ref_tokens;
  flows[i].ref_files=ref_files;
 }
 flows
}
#[cfg(test)]mod tests{
 use super::*;
 fn graph()->Graph{Graph{root:String::new(),entries:vec![],nodes:vec![crate::core::Node{id:"child.md".into(),label:"child.md".into(),kind:"document".into(),title:"子流程".into(),summary:String::new(),headings:vec![]}],edges:vec![],warnings:vec![],flows:vec![]}}
 #[test]fn excludes_code_fence_headings_and_preserves_all_h2_sections_with_budgets(){
  let source="## Introduction\n[ref](child.md)\n## Phase 0：门禁\n执行检查。\n[child](child.md)\n```md\n## Step fake\n```\n## Step 1：Review\n[child](child.md)\n## Appendix\nnotes\n";
  let mut graph=graph();for line in [2,5,10]{graph.edges.push(crate::core::Edge{source:"SKILL.md".into(),target:"child.md".into(),line,missing:false});}
  let flow=parse("SKILL.md",source,"主技能",true,&graph);
  assert_eq!(flow.stages.len(),4);
  assert!(!flow.stages[0].explicit&&flow.stages[1].explicit&&flow.stages[2].explicit&&!flow.stages[3].explicit);
  assert_eq!(flow.stages[1].line,3);assert_eq!(flow.stages[2].line,9);
  assert_eq!(flow.stages[1].calls[0].line,5);assert_eq!(flow.stages[2].calls[0].line,10);
  assert!(flow.total_tokens>0&&flow.stages.iter().all(|s|s.tokens>0));
 }
 #[test]fn entry_semantics_do_not_leak_from_later_stages(){
  let source="# English Title\n## 使命\n构建知识库。\n## Step 1：分析\n输出：分析结果。\n";
  let flow=parse("SKILL.md",source,"English Title",true,&graph());assert!(flow.meaning.output.is_none());assert!(flow.stages[1].meaning.output.is_some());
 }
 #[test]fn unnumbered_child_protocol_uses_h2_sections(){let flow=parse("child.md","# 协议\n## A. 检查\ncheck\n## B. 等待确认\nwait\n","协议",false,&graph());assert_eq!(flow.stages.len(),2);}
}
