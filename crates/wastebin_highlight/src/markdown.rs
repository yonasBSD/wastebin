use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd, html,
};

use crate::highlight::Error;
use crate::{Highlighter, Html};

/// Render CommonMark `text` to HTML. Fenced code blocks with a known language are syntax
/// highlighted via `highlighter`; unknown languages fall back to plain text. Raw HTML is
/// disabled — any `<script>`-like input is rendered as visible text.
pub fn render(text: &str, highlighter: &Highlighter) -> Result<Html, Error> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM;

    let parser = Parser::new_ext(text, options);
    let events = rewrite_events(parser, highlighter)?;

    let mut out = String::with_capacity(text.len());
    html::push_html(&mut out, events.into_iter());
    Ok(Html::new(out))
}

fn rewrite_events<'a>(
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
            Event::Start(Tag::BlockQuote(Some(kind))) => {
                out.push(Event::Start(Tag::BlockQuote(Some(kind))));
                out.push(Event::Html(CowStr::from(alert_title(kind))));
            }
            Event::Html(raw) | Event::InlineHtml(raw) => {
                out.push(Event::Text(raw));
            }
            other => out.push(other),
        }
    }

    Ok(out)
}

/// Return the HTML injected at the top of a GFM alert blockquote.
fn alert_title(kind: BlockQuoteKind) -> String {
    let label = match kind {
        BlockQuoteKind::Note => "Note",
        BlockQuoteKind::Tip => "Tip",
        BlockQuoteKind::Important => "Important",
        BlockQuoteKind::Warning => "Warning",
        BlockQuoteKind::Caution => "Caution",
    };
    format!("<p class=\"markdown-alert-title\">{label}</p>")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_string(text: &str, highlighter: &Highlighter) -> Result<String, Error> {
        render(text, highlighter).map(Html::into_inner)
    }

    #[test]
    fn heading() -> Result<(), Box<dyn std::error::Error>> {
        let html = render_string("# Hello", &Highlighter::default())?;
        assert!(html.contains("<h1>Hello</h1>"), "got: {html}");
        Ok(())
    }

    #[test]
    fn table() -> Result<(), Box<dyn std::error::Error>> {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let html = render_string(md, &Highlighter::default())?;
        assert!(html.contains("<table>"), "got: {html}");
        assert!(html.contains("<th>a</th>"), "got: {html}");
        Ok(())
    }

    #[test]
    fn task_list() -> Result<(), Box<dyn std::error::Error>> {
        let html = render_string("- [x] done\n- [ ] open\n", &Highlighter::default())?;
        assert!(html.contains("type=\"checkbox\""), "got: {html}");
        assert!(html.contains("checked"), "got: {html}");
        Ok(())
    }

    #[test]
    fn strikethrough() -> Result<(), Box<dyn std::error::Error>> {
        let html = render_string("~~gone~~", &Highlighter::default())?;
        assert!(html.contains("<del>gone</del>"), "got: {html}");
        Ok(())
    }

    #[test]
    fn code_block_is_highlighted() -> Result<(), Box<dyn std::error::Error>> {
        let md = "```rust\nfn main() {}\n```\n";
        let html = render_string(md, &Highlighter::default())?;
        assert!(html.contains("class=\"code-block language-rust\""), "got: {html}");
        assert!(html.contains("<span class=\""), "got: {html}");
        Ok(())
    }

    #[test]
    fn code_block_unknown_language_falls_back() -> Result<(), Box<dyn std::error::Error>> {
        let md = "```not-a-real-lang\nhello\n```\n";
        let html = render_string(md, &Highlighter::default())?;
        assert!(html.contains("<pre"), "got: {html}");
        assert!(html.contains("hello"), "got: {html}");
        Ok(())
    }

    #[test]
    fn code_block_without_language() -> Result<(), Box<dyn std::error::Error>> {
        let md = "```\nraw\n```\n";
        let html = render_string(md, &Highlighter::default())?;
        assert!(html.contains("class=\"code-block\""), "got: {html}");
        assert!(!html.contains("language-"), "got: {html}");
        Ok(())
    }

    #[test]
    fn code_block_malicious_language_is_sanitized() -> Result<(), Box<dyn std::error::Error>> {
        let md = "```\"><script>\ncode\n```\n";
        let html = render_string(md, &Highlighter::default())?;
        assert!(!html.contains("<script>"), "got: {html}");
        Ok(())
    }

    #[test]
    fn gfm_alert_note() -> Result<(), Box<dyn std::error::Error>> {
        let md = "> [!NOTE]\n> body\n";
        let html = render_string(md, &Highlighter::default())?;
        assert!(
            html.contains("<blockquote class=\"markdown-alert-note\">"),
            "got: {html}"
        );
        assert!(
            html.contains("<p class=\"markdown-alert-title\">Note</p>"),
            "got: {html}"
        );
        assert!(html.contains("body"), "got: {html}");
        Ok(())
    }

    #[test]
    fn gfm_alert_variants() -> Result<(), Box<dyn std::error::Error>> {
        for (marker, class, label) in [
            ("TIP", "markdown-alert-tip", "Tip"),
            ("IMPORTANT", "markdown-alert-important", "Important"),
            ("WARNING", "markdown-alert-warning", "Warning"),
            ("CAUTION", "markdown-alert-caution", "Caution"),
        ] {
            let md = format!("> [!{marker}]\n> body\n");
            let html = render_string(&md, &Highlighter::default())?;
            assert!(html.contains(class), "{marker}: {html}");
            assert!(
                html.contains(&format!(
                    "<p class=\"markdown-alert-title\">{label}</p>"
                )),
                "{marker}: {html}"
            );
        }
        Ok(())
    }

    #[test]
    fn plain_blockquote_is_not_an_alert() -> Result<(), Box<dyn std::error::Error>> {
        let html = render_string("> just a quote\n", &Highlighter::default())?;
        assert!(html.contains("<blockquote>"), "got: {html}");
        assert!(!html.contains("markdown-alert"), "got: {html}");
        Ok(())
    }

    #[test]
    fn raw_html_is_escaped() -> Result<(), Box<dyn std::error::Error>> {
        let html = render_string("<script>alert(1)</script>", &Highlighter::default())?;
        assert!(!html.contains("<script>"), "got: {html}");
        assert!(html.contains("&lt;script&gt;"), "got: {html}");
        Ok(())
    }
}
