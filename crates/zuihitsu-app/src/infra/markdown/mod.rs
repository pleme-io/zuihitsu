//! Markdown rendering. Hashnode returns both markdown and pre-rendered HTML,
//! so this module is only needed when we want to re-render (e.g. for RSS).

#[cfg(feature = "ssr")]
pub fn render_html(markdown: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}
