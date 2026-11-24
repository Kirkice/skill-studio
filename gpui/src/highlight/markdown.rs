use gpui::{HighlightStyle, rgb};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::{collections::BTreeSet, ops::Range};
use syntect::{easy::HighlightLines, highlighting::Theme, parsing::SyntaxSet};

type Run = (Range<usize>, HighlightStyle);

pub(super) fn highlight(text: &str, light: bool, syntaxes: &SyntaxSet, theme: &Theme) -> Vec<Run> {
    let color = |day, night| HighlightStyle {
        color: Some(rgb(if light { day } else { night }).into()),
        ..Default::default()
    };
    let muted = color(0x596676, 0x9aa9be);
    let marker = color(0x805829, 0xd8be87);
    let link = color(0x155d93, 0x96c8ef);
    let code = color(0x8b3453, 0xe5a6bf);
    let mut runs = Vec::new();
    let mut add = |range: Range<usize>, style| {
        if !range.is_empty() {
            runs.push((range, style));
        }
    };
    let mut block = None;
    let mut metadata = false;
    let options = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH;
    for (event, range) in Parser::new_ext(text, options).into_offset_iter() {
        let raw = &text[range.clone()];
        match event {
            Event::Start(Tag::MetadataBlock(_)) => {
                metadata = true;
                for marker in [raw.lines().next(), raw.lines().last()]
                    .into_iter()
                    .flatten()
                {
                    if marker.trim() == "---" {
                        let base = if marker.as_ptr() == raw.as_ptr() {
                            range.start
                        } else {
                            range.start + raw.rfind(marker).unwrap()
                        };
                        add(base..base + marker.len(), muted);
                    }
                }
            }
            Event::End(TagEnd::MetadataBlock(_)) => metadata = false,
            Event::Text(_) if metadata => {
                let mut offset = range.start;
                for line in raw.split_inclusive('\n') {
                    let key = line.trim_start();
                    if let Some((key, _)) = key.split_once(':') {
                        if !key.is_empty()
                            && key
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                        {
                            let start = offset + line.len() - line.trim_start().len();
                            add(start..start + key.len(), link);
                        }
                    }
                    offset += line.len();
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match &kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or(""),
                    _ => "",
                };
                if matches!(kind, CodeBlockKind::Fenced(_)) {
                    add(range, muted);
                }
                let syntax = syntaxes
                    .find_syntax_by_token(language)
                    .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
                block = Some(HighlightLines::new(syntax, theme));
            }
            Event::End(TagEnd::CodeBlock) => block = None,
            Event::Text(_) if block.is_some() => {
                let mut offset = range.start;
                let highlighter = block.as_mut().unwrap();
                for line in raw.split_inclusive('\n') {
                    if let Ok(parts) = highlighter.highlight_line(line, syntaxes) {
                        for (style, span) in parts {
                            let c = style.foreground;
                            let token = HighlightStyle {
                                color: Some(rgb(u32::from_be_bytes([0, c.r, c.g, c.b])).into()),
                                ..Default::default()
                            };
                            add(offset..offset + span.len(), token);
                            offset += span.len();
                        }
                    } else {
                        offset += line.len();
                    }
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let style = match level {
                    HeadingLevel::H1 => color(0x244d85, 0xaecafa),
                    HeadingLevel::H2 => color(0x60448d, 0xcbb5ef),
                    _ => color(0x286660, 0x9cd0c6),
                };
                add(range, style);
            }
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => add(range, link),
            Event::Code(_) => add(range, code),
            Event::TaskListMarker(_) => add(range, marker),
            Event::Start(Tag::Item | Tag::BlockQuote(_)) => {
                let start = range.start + raw.len() - raw.trim_start().len();
                let prefix = text[start..range.end]
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                if prefix == ">"
                    || matches!(prefix, "-" | "+" | "*")
                    || prefix
                        .strip_suffix(['.', ')'])
                        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                {
                    add(start..start + prefix.len(), marker);
                } else if prefix.starts_with('>') {
                    add(start..start + 1, marker);
                }
            }
            _ => {}
        }
    }
    // Nested syntax (e.g. inline code in a heading) takes precedence.
    let mut edges = Vec::with_capacity(2 * runs.len());
    for (i, (range, _)) in runs.iter().enumerate() {
        edges.push((range.start, i, true));
        edges.push((range.end, i, false));
    }
    edges.sort_unstable();
    let mut active = BTreeSet::<usize>::new();
    let mut result: Vec<Run> = Vec::new();
    let mut previous = 0;
    for (offset, i, start) in edges {
        if previous < offset
            && let Some(&selected) = active.last()
        {
            let style = runs[selected].1;
            if let Some((range, last)) = result
                .last_mut()
                .filter(|(r, s)| r.end == previous && *s == style)
            {
                let _ = last;
                range.end = offset;
            } else {
                result.push((previous..offset, style));
            }
        }
        if start {
            active.insert(i);
        } else {
            active.remove(&i);
        }
        previous = offset;
    }
    result
}
