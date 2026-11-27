//! Conservative, source-backed card semantics. No model-generated claims.
use pulldown_cmark::{Event,Parser,Tag,TagEnd};
use serde::Serialize;
#[derive(Clone,Default,Serialize)]
pub struct Excerpt{pub text:String,pub line:usize}
#[derive(Clone,Default,Serialize)]
pub struct Meaning{
 pub action:Excerpt,pub purpose:Option<Excerpt>,pub purpose_explicit:bool,
 pub condition:Option<Excerpt>,pub confirmation:Option<Excerpt>,pub input:Option<Excerpt>,pub output:Option<Excerpt>,
}
struct Block{value:Excerpt,heading:String}
fn short(text:&str,limit:usize)->String{let text=text.trim();let mut out:String=text.chars().take(limit).collect();if text.chars().count()>limit{out.push('…');}out}
fn is_ascii_noise(text:&str)->bool{
 let mut run=0usize;
 for c in text.chars(){
  if matches!(c,'\u{2500}'..='\u{257f}'|'—'|'-'|'='|'_'|'*'){
   run+=1;if run>=6{return true;}
  }else{run=0;}
 }
 false
}
fn blocks(source:&str,base_line:usize)->Vec<Block>{
 let body=crate::preview::body(source);let offset=source.len()-body.len();
 let base_line=base_line+source[..offset].bytes().filter(|b|*b==b'\n').count();let source=&source[offset..];
 let newlines:Vec<_>=source.bytes().enumerate().filter_map(|(i,b)|(b==b'\n').then_some(i)).collect();
 let line_at=|offset|base_line+newlines.partition_point(|&i|i<offset);
 let mut result=vec![];let mut heading=String::new();let mut active:Option<(Excerpt,bool)>=None;
 let mut in_item=false;
 for(event,range)in Parser::new(source).into_offset_iter(){match event{
  Event::Start(Tag::Heading{..})=>active=Some((Excerpt{text:String::new(),line:line_at(range.start)},true)),
  Event::Start(Tag::Item)=>{in_item=true;if active.is_none(){active=Some((Excerpt{text:String::new(),line:line_at(range.start)},false));}},
  Event::Start(Tag::Paragraph)=>if active.is_none(){active=Some((Excerpt{text:String::new(),line:line_at(range.start)},false));},
  Event::Text(t)|Event::Code(t)=>if let Some((value,_))=&mut active{value.text.push_str(&t);},
  Event::SoftBreak|Event::HardBreak=>if let Some((value,_))=&mut active{value.text.push('\n');},
  Event::End(TagEnd::Heading(_))=>if let Some((value,_))=active.take(){heading=value.text;},
  Event::End(TagEnd::Paragraph)=>if !in_item{if let Some((mut value,_))=active.take(){value.text=value.text.trim().to_string();if !value.text.is_empty(){result.push(Block{value,heading:heading.clone()});}}},
  Event::End(TagEnd::Item)=>{in_item=false;if let Some((mut value,_))=active.take(){value.text=value.text.trim().to_string();if !value.text.is_empty(){result.push(Block{value,heading:heading.clone()});}}},
  _=>{}
 }}result
}
fn sentence(value:&Excerpt,limit:usize)->Excerpt{let text=value.text.replace('\n'," ");Excerpt{text:short(text.split('。').find(|s|!s.trim().is_empty()).unwrap_or(&text),limit),line:value.line}}
fn label(text:&str,names:&[&str])->Option<String>{
 let text=text.trim();for name in names{if let Some(rest)=text.strip_prefix(name){let rest=rest.trim_start();if let Some(rest)=rest.strip_prefix(['：',':']){return Some(rest.trim().into());}if let Some(rest)=rest.strip_prefix('为'){return Some(rest.trim().into());}}}None
}
fn negative(text:&str)->bool{["无需用户确认","不需要用户确认","不必用户确认","不用用户确认","无需确认","不需要确认","无需再确认","不必确认"].iter().any(|s|text.contains(s))}
fn field(blocks:&[Block],names:&[&str])->Option<Excerpt>{
 for block in blocks{
  for(offset,line)in block.value.text.lines().enumerate(){if let Some(text)=label(line,names){return Some(sentence(&Excerpt{text,line:block.value.line+offset},86));}}
  if names.iter().any(|name|block.heading.trim()==*name){return Some(sentence(&block.value,86));}
 }None
}
fn cleaned_title(title:&str)->String{
 let lower=title.to_lowercase();
 if ["step ","phase ","阶段","步骤"].iter().any(|p|lower.starts_with(p)){
  if let Some((_,body))=title.split_once(['：',':']){return body.trim().into();}
 }
 title.trim().into()
}
pub fn document(title:&str,source:&str)->Meaning{
 let mut meaning=extract(title,1,source,1);let items=blocks(source,1);
 // A route mentioned in a Skill's overview is not the entry condition of the entire Skill.
 meaning.condition=field(&items,&["触发条件","触发","执行条件","前提","条件","trigger condition","trigger","precondition"]);
 meaning.confirmation=field(&items,&["确认要求","用户确认"]);meaning
}
pub fn extract(title:&str,title_line:usize,source:&str,base_line:usize)->Meaning{
 let items=blocks(source,base_line);
 let purpose_block=items.iter().find(|b|["使命","目的","目标","用途","purpose","mission","goal"].contains(&b.heading.to_lowercase().trim()))
  .or_else(||items.iter().find(|b|label(&b.value.text,&["目的","目标","用途"]).is_some()))
  .or_else(||items.iter().find(|b|!is_ascii_noise(&b.value.text)&&!b.value.text.starts_with("本步骤不是")&&field(std::slice::from_ref(b),&["输入","输出","返回"]).is_none()&&b.value.text.chars().filter(|c|c.is_alphabetic()).count()>6));
 let explicit_purpose=purpose_block.is_some_and(|b|["使命","目的","目标","用途","purpose","mission","goal"].contains(&b.heading.to_lowercase().trim())||label(&b.value.text,&["目的","目标","用途"]).is_some());
 let purpose=purpose_block.map(|b|{let text=label(&b.value.text,&["目的","目标","用途"]).unwrap_or_else(||b.value.text.clone());sentence(&Excerpt{text,line:b.value.line},120)});
 let clean=cleaned_title(title);
 let explicit_action=field(&items,&["动作","操作","任务"]);
 let action=if let Some(action)=explicit_action{action}
 else if !clean.chars().any(|c|('\u{4e00}'..='\u{9fff}').contains(&c))&&explicit_purpose{
  let mut action=purpose.clone().unwrap();action.text=short(&action.text,48);action
 }else{Excerpt{text:clean,line:title_line}};
 let condition=field(&items,&["触发条件","触发","执行条件","前提","条件","trigger condition","trigger","precondition"]).or_else(||{
  for (open,close)in [('（','）'),('(',')')]{if let Some(start)=title.find(open){if let Some(end)=title[start..].find(close){let text=&title[start+open.len_utf8()..start+end];if text.contains("仅")||text.contains("only"){return Some(Excerpt{text:text.into(),line:title_line});}}}}
  items.iter().find(|b|["仅当","只有","当用户","用户同意","用户明确要求"].iter().any(|p|b.value.text.starts_with(p))).map(|b|sentence(&b.value,90))
 });
 let confirmation=if title.contains("用户确认")||title.contains("用户授权"){Some(Excerpt{text:title.into(),line:title_line})}else{
  items.iter().find(|b|!negative(&b.value.text)&&["等待用户","等待确认","用户确认后","用户明确同意","用户明确表示","提交给用户确认","用户确认前","用户首次确认前","用户首次确认"].iter().any(|p|b.value.text.contains(p))).map(|b|{
   let markers=["等待用户","等待确认","用户确认后","用户明确同意","用户明确表示","提交给用户确认","用户确认前","用户首次确认前","用户首次确认"];
   let text=b.value.text.split(['。','；']).find(|text|markers.iter().any(|marker|text.contains(marker))).unwrap_or(&b.value.text);
   Excerpt{text:short(text,100),line:b.value.line}
  })
 };
 Meaning{action,purpose,purpose_explicit:explicit_purpose,condition,confirmation,input:field(&items,&["输入","输入契约","输入要求"]),output:field(&items,&["输出","返回","输出契约","产出"])}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn explicit_semantics_have_source_lines_and_ignore_fenced_examples(){
  let text="## 目标\n检查项目知识库，避免无依据生成。\n\n触发条件：知识库缺失或失效。\n\n输入：已确认目录。\n\n输出：知识库版本。\n\n等待用户确认后继续。\n```md\n输入：伪造输入\n```";
  let m=extract("Phase 0：检查知识库",10,text,11);
  assert_eq!(m.action.text,"检查知识库");assert_eq!(m.action.line,10);assert!(m.purpose_explicit);
  assert_eq!(m.purpose.unwrap().line,12);assert!(m.condition.unwrap().text.contains("缺失"));
  assert_eq!(m.input.unwrap().line,16);assert!(m.output.unwrap().text.contains("版本"));assert!(m.confirmation.is_some());
 }
 #[test]fn no_missing_fields_are_invented_or_negative_confirmation_promoted(){
  let m=extract("Step 2：生成报告（仅 PBR 路线）",20,"无需用户确认。\n\n```\n输出：样例\n```",21);
  assert!(m.input.is_none()&&m.output.is_none()&&m.confirmation.is_none());assert_eq!(m.condition.unwrap().line,20);
 }
 #[test]fn prohibition_before_confirmation_is_still_a_confirmation_gate(){
  let m=extract("Step 6：确认门禁",10,"用户首次确认前，禁止调用写入工具。",11);assert!(m.confirmation.is_some());
 }
 #[test]fn adjacent_input_output_lines_keep_distinct_evidence(){
  let m=extract("Step 1：检查目录",1,"输入：项目目录\n输出：文件清单\n",2);
  assert_eq!(m.input.unwrap().text,"项目目录");let output=m.output.unwrap();assert_eq!(output.text,"文件清单");assert_eq!(output.line,3);
 }
 #[test]fn frontmatter_is_not_a_purpose_and_offsets_remain_correct(){
  let m=document("Title","---\nname: tool\ndescription: metadata\n---\n# Title\n\n这里介绍如何检查本地配置并生成报告。\n");
  let purpose=m.purpose.unwrap();assert!(!purpose.text.contains("metadata"));assert_eq!(purpose.line,7);
 }
 #[test]fn english_skill_title_uses_explicit_chinese_mission(){
  let m=extract("Knowledge Builder",1,"# Knowledge Builder\n## 使命\n构建项目知识库，为后续编写提供事实。",1);
  assert!(m.action.text.starts_with("构建"));assert_eq!(m.action.line,3);
 }
}
