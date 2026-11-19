use gpui::{Context, HighlightStyle, SharedString, Window, rgb};
use gpui_base::input::{EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter, RopeExt};
use ropey::Rope;
use std::{ops::Range, sync::OnceLock};
use syntect::{easy::HighlightLines, highlighting::{HighlightState, ThemeSet}, parsing::{ParseState, SyntaxSet}};

type ParserSnapshot = (HighlightState, ParseState);
struct CachedLine {
    text: String,
    runs: Vec<(Range<usize>, HighlightStyle)>,
    after: ParserSnapshot,
}
pub struct Highlighter {
    language: String,
    lines: Vec<CachedLine>,
    offsets: Vec<usize>,
    light: Option<bool>,
    // Also used by regression tests to ensure unchanged suffixes aren't reparsed.
    parsed_lines: usize,
}
impl Highlighter {
    pub fn new(language: &str) -> Self {
        Self {language:language.into(),lines:vec![],offsets:vec![],light:None,parsed_lines:0}
    }
    fn update_rope(&mut self, edit:Option<InputEdit>, text:&Rope, force:bool, light:bool) {
        self.parsed_lines=0;
        // Synchronous regex parsing is unsuitable for huge/pasted documents.
        // Keep editing responsive with plain text above these explicit limits.
        if text.len() > 512_000 || text.lines_len() > 12_000 {
            self.lines.clear();self.offsets.clear();self.light=None;return;
        }
        static SYNTAX:OnceLock<SyntaxSet>=OnceLock::new();
        static THEMES:OnceLock<ThemeSet>=OnceLock::new();
        let syntaxes=SYNTAX.get_or_init(SyntaxSet::load_defaults_newlines);
        let themes=THEMES.get_or_init(ThemeSet::load_defaults);
        let syntax=syntaxes.find_syntax_by_extension(&self.language).unwrap_or_else(||syntaxes.find_syntax_plain_text());
        let theme=&themes.themes[if light{"InspiredGitHub"}else{"base16-ocean.dark"}];
        let incremental=edit.filter(|e| !force && self.light==Some(light) && !self.lines.is_empty()
            && e.start_position.row<=e.old_end_position.row && e.start_position.row<=e.new_end_position.row
            && self.lines.len().checked_add_signed(e.new_end_position.row as isize-e.old_end_position.row as isize)==Some(text.lines_len())
            && e.start_position.row<self.lines.len());
        let start=incremental.map_or(0,|e|e.start_position.row);
        let old=self.lines.split_off(start);
        let mut old:Vec<Option<CachedLine>>=old.into_iter().map(Some).collect();
        let mut state=self.lines.last().map(|line|line.after.clone()).unwrap_or_else(||HighlightLines::new(syntax,theme).state());
        for row in start..text.lines_len() {
            let line=text.slice_line(row).to_string();
            let mut h=HighlightLines::from_state(theme,state.0.clone(),state.1.clone());
            let mut offset=0;
            let runs=match h.highlight_line(&line,syntaxes) {
                Ok(parts)=>parts.into_iter().map(|(style,span)|{
                    let range=offset..offset+span.len();offset=range.end;let c=style.foreground;
                    (range,HighlightStyle{color:Some(rgb((u32::from(c.r)<<16)|(u32::from(c.g)<<8)|u32::from(c.b)).into()),..Default::default()})
                }).collect(),
                Err(_)=>vec![(0..line.len(),HighlightStyle::default())],
            };
            state=h.state();self.parsed_lines+=1;
            let cached=CachedLine{text:line,runs,after:state.clone()};
            let aligned=incremental.and_then(|e| {
                if row<=e.new_end_position.row{return None;}
                let old_row=row.checked_add_signed(e.old_end_position.row as isize-e.new_end_position.row as isize)?;
                old_row.checked_sub(start)
            });
            let converged=aligned.is_some_and(|i| old.get(i).and_then(Option::as_ref).is_some_and(|previous|previous.text==cached.text && previous.after==cached.after));
            self.lines.push(cached);
            if converged {
                self.lines.extend(old.iter_mut().skip(aligned.unwrap()+1).filter_map(Option::take));
                break;
            }
        }
        self.offsets.clear();let mut offset=0;
        for line in &self.lines {self.offsets.push(offset);offset+=line.text.len();}
        self.light=Some(light);
    }
    fn styles_in(&self,range:&Range<usize>)->Vec<(Range<usize>,HighlightStyle)> {
        if range.is_empty(){return vec![];}
        let first=self.offsets.partition_point(|offset|*offset<=range.start).saturating_sub(1);
        let mut result=vec![];
        for row in first..self.lines.len() {
            let offset=self.offsets[row];if offset>=range.end{break;}
            for (r,style) in &self.lines[row].runs {
                let start=(offset+r.start).max(range.start);let end=(offset+r.end).min(range.end);
                if start<end{result.push((start..end,*style));}
            }
        }
        result
    }
}
impl InputHighlighter for Highlighter {
    fn language(&self)->SharedString{self.language.clone().into()}
    fn update(&mut self,edit:Option<InputEdit>,text:&Rope,_folding:bool,_:&mut Window,_:&mut Context<EditorState>){self.update_rope(edit,text,false,crate::ui::theme::is_light());}
    fn styles(&self,range:&Range<usize>,_:&dyn HighlightStyleResolver)->Vec<(Range<usize>,HighlightStyle)>{self.styles_in(range)}
    fn fold_ranges(&self,_:&Rope)->Vec<FoldRange>{vec![]}
}
#[cfg(test)]mod tests {
    use super::*;
    use gpui_base::input::Point;
    fn edit(start:usize,old:usize,new:usize)->InputEdit{InputEdit{start_byte:0,old_end_byte:0,new_end_byte:0,start_position:Point::new(start,0),old_end_position:Point::new(old,0),new_end_position:Point::new(new,0)}}
    fn compare(incremental:&Highlighter,text:&str){let mut full=Highlighter::new("rs");full.update_rope(None,&Rope::from_str(text),true,false);assert_eq!(incremental.styles_in(&(0..text.len())),full.styles_in(&(0..text.len())));}
    #[test]fn unchanged_suffix_is_reused(){
        let before=(0..1000).map(|i|format!("let x{i} = 1;\n")).collect::<String>();
        let after=before.replacen("x500 = 1","x500 = 2",1);
        let mut h=Highlighter::new("rs");h.update_rope(None,&Rope::from_str(&before),false,false);
        h.update_rope(Some(edit(500,500,500)),&Rope::from_str(&after),false,false);
        assert!(h.parsed_lines<=3,"reparsed {} lines",h.parsed_lines);compare(&h,&after);
    }
    #[test]fn multiline_state_and_line_insert_delete_match_full_parse(){
        let mut h=Highlighter::new("rs");h.update_rope(None,&Rope::from_str("let a=1;\n中文\nlet b=2;\n"),false,false);
        let after="/*\nlet a=1;\n中文\n*/\nlet b=2;\n";
        h.update_rope(Some(edit(0,1,3)),&Rope::from_str(after),false,false);compare(&h,after);
        let after="let a=1;\n中文\nlet b=2;\n";
        h.update_rope(Some(edit(0,3,1)),&Rope::from_str(after),false,false);compare(&h,after);
    }
    #[test]fn large_documents_use_plain_text(){let mut h=Highlighter::new("rs");h.update_rope(None,&Rope::from_str(&"x".repeat(512001)),false,false);assert!(h.styles_in(&(0..10)).is_empty());assert_eq!(h.parsed_lines,0);}
}
