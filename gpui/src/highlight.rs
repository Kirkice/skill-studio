use gpui::{Context, HighlightStyle, SharedString, Window, rgb};
use gpui_base::input::{EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter};
use ropey::Rope;
use std::{ops::Range, sync::OnceLock};
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

pub struct Highlighter { language: String, runs: Vec<(Range<usize>, HighlightStyle)> }
impl Highlighter { pub fn new(language: &str) -> Self { Self {language:language.into(), runs:vec![]} } }
impl InputHighlighter for Highlighter {
 fn language(&self) -> SharedString { self.language.clone().into() }
 fn update(&mut self, _:Option<InputEdit>, text:&Rope, _:bool, _: &mut Window, _: &mut Context<EditorState>) {
  static SYNTAX: OnceLock<SyntaxSet> = OnceLock::new();
  static THEMES: OnceLock<ThemeSet> = OnceLock::new();
  let syntaxes=SYNTAX.get_or_init(SyntaxSet::load_defaults_newlines);
  let themes=THEMES.get_or_init(ThemeSet::load_defaults);
  let syntax=syntaxes.find_syntax_by_extension(&self.language).unwrap_or_else(|| syntaxes.find_syntax_plain_text());
  let theme=&themes.themes[if crate::ui::theme::is_light(){"InspiredGitHub"}else{"base16-ocean.dark"}];
  let mut highlighter=HighlightLines::new(syntax,theme);
  self.runs.clear(); let mut offset=0;
  let content=text.to_string();
  for line in content.split_inclusive('\n') {
   match highlighter.highlight_line(line,syntaxes) {
    Ok(parts)=>for (style,span) in parts { let end=offset+span.len(); let c=style.foreground; self.runs.push((offset..end,HighlightStyle{color:Some(rgb((u32::from(c.r)<<16)|(u32::from(c.g)<<8)|u32::from(c.b)).into()),..Default::default()})); offset=end; },
    Err(_)=>{let end=offset+line.len();self.runs.push((offset..end,HighlightStyle::default()));offset=end;}
   }
  }
 }
 fn styles(&self, range:&Range<usize>, _: &dyn HighlightStyleResolver)->Vec<(Range<usize>,HighlightStyle)> {
  self.runs.iter().filter_map(|(r,s)| {let start=r.start.max(range.start);let end=r.end.min(range.end);(start<end).then_some((start..end,*s))}).collect()
 }
 fn fold_ranges(&self,_:&Rope)->Vec<FoldRange> {vec![]}
}
