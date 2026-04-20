use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd, html};

use crate::Highlighter;
use crate::highlight::Error;

/// Render CommonMark `text` to HTML. Fenced code blocks with a known language are syntax
/// highlighted via `highlighter`; unknown languages fall back to plain text. Raw HTML is
/// disabled — any `<script>`-like input is rendered as visible text.
pub fn render(text: &str, highlighter: &Highlighter) -> Result<String, Error> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;

    let parser = Parser::new_ext(text, options);
    let events = rewrite_code_blocks(parser, highlighter)?;

    let mut out = String::with_capacity(text.len());
    html::push_html(&mut out, events.into_iter());
    Ok(out)
}

fn rewrite_code_blocks<'a>(
    parser: Parser<'a>,
    highlighter: &Highlighter,
) -> Result<Vec<Event<'a>>, Error> {
    let mut out = Vec::new();
    let mut pending: Option<(String, String)> = None;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                pending = Some((lang.to_string(), String::new()));
            }
            Event::Text(text) if pending.is_some() => {
                if let Some((_, buf)) = pending.as_mut() {
                    buf.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) if pending.is_some() => {
                if let Some((lang, code)) = pending.take() {
                    let html = highlighter.highlight_code_block(&code, &lang)?;
                    out.push(Event::Html(CowStr::from(html)));
                }
            }
            Event::Html(raw) | Event::InlineHtml(raw) => {
                out.push(Event::Text(raw));
            }
            other => out.push(other),
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading() -> Result<(), Box<dyn std::error::Error>> {
        let html = render("# Hello", &Highlighter::default())?;
        assert!(html.contains("<h1>Hello</h1>"), "got: {html}");
        Ok(())
    }

    #[test]
    fn table() -> Result<(), Box<dyn std::error::Error>> {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let html = render(md, &Highlighter::default())?;
        assert!(html.contains("<table>"), "got: {html}");
        assert!(html.contains("<th>a</th>"), "got: {html}");
        Ok(())
    }

    #[test]
    fn task_list() -> Result<(), Box<dyn std::error::Error>> {
        let html = render("- [x] done\n- [ ] open\n", &Highlighter::default())?;
        assert!(html.contains("type=\"checkbox\""), "got: {html}");
        assert!(html.contains("checked"), "got: {html}");
        Ok(())
    }

    #[test]
    fn strikethrough() -> Result<(), Box<dyn std::error::Error>> {
        let html = render("~~gone~~", &Highlighter::default())?;
        assert!(html.contains("<del>gone</del>"), "got: {html}");
        Ok(())
    }

    #[test]
    fn code_block_is_highlighted() -> Result<(), Box<dyn std::error::Error>> {
        let md = "```rust\nfn main() {}\n```\n";
        let html = render(md, &Highlighter::default())?;
        assert!(html.contains("class=\"code-block language-rust\""), "got: {html}");
        assert!(html.contains("<span class=\""), "got: {html}");
        Ok(())
    }

    #[test]
    fn code_block_unknown_language_falls_back() -> Result<(), Box<dyn std::error::Error>> {
        let md = "```not-a-real-lang\nhello\n```\n";
        let html = render(md, &Highlighter::default())?;
        assert!(html.contains("<pre"), "got: {html}");
        assert!(html.contains("hello"), "got: {html}");
        Ok(())
    }

    #[test]
    fn code_block_without_language() -> Result<(), Box<dyn std::error::Error>> {
        let md = "```\nraw\n```\n";
        let html = render(md, &Highlighter::default())?;
        assert!(html.contains("class=\"code-block\""), "got: {html}");
        assert!(!html.contains("language-"), "got: {html}");
        Ok(())
    }

    #[test]
    fn code_block_malicious_language_is_sanitized() -> Result<(), Box<dyn std::error::Error>> {
        let md = "```\"><script>\ncode\n```\n";
        let html = render(md, &Highlighter::default())?;
        assert!(!html.contains("<script>"), "got: {html}");
        Ok(())
    }

    #[test]
    fn raw_html_is_escaped() -> Result<(), Box<dyn std::error::Error>> {
        let html = render("<script>alert(1)</script>", &Highlighter::default())?;
        assert!(!html.contains("<script>"), "got: {html}");
        assert!(html.contains("&lt;script&gt;"), "got: {html}");
        Ok(())
    }
}
