//! Evidence-backed document-order candidates, not an executable workflow.
use crate::core::Graph;
use pulldown_cmark::{Event,HeadingLevel,Parser,Tag,TagEnd};
use serde::Serialize;
use std::{collections::HashSet,path::Path};
#[derive(Clone,Serialize)]
pub struct Call{pub target:String,pub title:String,pub line:usize}
#[derive(Clone,Serialize)]
pub struct Stage{pub title:String,pub line:usize,pub summary:String,pub calls:Vec<Call>,pub meaning:crate::meaning::Meaning}
#[derive(Clone,Serialize)]
pub struct Flow{pub source:String,pub title:String,pub entry:bool,pub stages:Vec<Stage>,pub meaning:crate::meaning::Meaning}
struct Heading{title:String,line:usize,start:usize,end:usize}
fn explicit(title:&str)->bool{
 let t=title.trim().to_lowercase();
 ["step ","phase ","阶段","步骤"].iter().any(|prefix|t.starts_with(prefix))
}
pub fn parse(source:&str,content:&str,title:&str,entry:bool,graph:&Graph)->Flow{
 let mut headings=vec![];let mut current:Option<Heading>=None;
 for(event,range)in Parser::new(content).into_offset_iter(){match event{
  Event::Start(Tag::Heading{level:HeadingLevel::H2,..})=>current=Some(Heading{title:String::new(),line:content[..range.start].bytes().filter(|b|*b==b'\n').count()+1,start:range.start,end:range.end}),
  Event::Text(text)|Event::Code(text)=>if let Some(h)=&mut current{h.title.push_str(&text);},
  Event::End(TagEnd::Heading(HeadingLevel::H2))=>if let Some(mut h)=current.take(){h.end=range.end;headings.push(h);},
  _=>{}
 }}
 let has_explicit=headings.iter().any(|h|explicit(&h.title));
 let mut stages=vec![];
 for(i,h)in headings.iter().enumerate(){
  // Explicit stage markers win. Otherwise expose sections as candidate stages.
  if has_explicit&&!explicit(&h.title){continue;}
  let end=headings.get(i+1).map_or(content.len(),|next|next.start);
  let next_line=headings.get(i+1).map_or(usize::MAX,|next|next.line);
  let mut seen=HashSet::new();
  let calls=graph.edges.iter().filter(|e|e.source==source&&e.line>h.line&&e.line<next_line&&!e.missing).filter_map(|edge|{
   let node=graph.nodes.iter().find(|n|n.id==edge.target&&matches!(n.kind.as_str(),"skill"|"document"))?;
   if !seen.insert(node.id.clone()){return None;}
   Some(Call{target:node.id.clone(),title:if node.title.is_empty(){node.label.clone()}else{node.title.clone()},line:edge.line})
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
  stages.push(Stage{title:h.title.clone(),line:h.line,summary:summary.chars().take(160).collect(),calls,meaning});
 }
 let entry_end=headings.iter().find(|h|explicit(&h.title)).map_or(content.len(),|h|h.start);
 Flow{source:source.into(),title:title.into(),entry,stages,meaning:crate::meaning::document(title,&content[..entry_end])}
}
pub fn collect(graph:&Graph)->Vec<Flow>{
 graph.nodes.iter().filter(|n|matches!(n.kind.as_str(),"skill"|"document")).filter_map(|node|{
  let content=crate::core::read_document(Path::new(&graph.root),&node.id).ok()?.content;
  let title=if node.title.is_empty(){&node.label}else{&node.title};
  Some(parse(&node.id,&content,title,node.kind=="skill",graph))
 }).collect()
}
#[cfg(test)]mod tests{
 use super::*;
 fn graph()->Graph{Graph{root:String::new(),entries:vec![],nodes:vec![crate::core::Node{id:"child.md".into(),label:"child.md".into(),kind:"document".into(),title:"子流程".into(),summary:String::new(),headings:vec![]}],edges:vec![],warnings:vec![],flows:vec![]}}
 #[test]fn excludes_code_fence_headings_and_preserves_repeated_stage_calls(){
  let source="## Introduction\n[ref](child.md)\n## Phase 0：门禁\n执行检查。\n[child](child.md)\n```md\n## Step fake\n```\n## Step 1：Review\n[child](child.md)\n## Appendix\nnotes\n";
  let mut graph=graph();for line in [2,5,10]{graph.edges.push(crate::core::Edge{source:"SKILL.md".into(),target:"child.md".into(),line,missing:false});}
  let flow=parse("SKILL.md",source,"主技能",true,&graph);
  assert_eq!(flow.stages.len(),2);assert_eq!(flow.stages[0].line,3);assert_eq!(flow.stages[1].line,9);
  assert_eq!(flow.stages[0].calls.len(),1);assert_eq!(flow.stages[1].calls.len(),1);
  assert_eq!(flow.stages[0].calls[0].line,5);assert_eq!(flow.stages[1].calls[0].line,10);
 }
 #[test]fn entry_semantics_do_not_leak_from_later_stages(){
  let source="# English Title\n## 使命\n构建知识库。\n## Step 1：分析\n输出：分析结果。\n";
  let flow=parse("SKILL.md",source,"English Title",true,&graph());assert!(flow.meaning.output.is_none());assert!(flow.stages[0].meaning.output.is_some());
 }
 #[test]fn unnumbered_child_protocol_uses_h2_sections(){let flow=parse("child.md","# 协议\n## A. 检查\ncheck\n## B. 等待确认\nwait\n","协议",false,&graph());assert_eq!(flow.stages.len(),2);}
}
