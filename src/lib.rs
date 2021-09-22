use anyhow::Result;
use reqwest::Url;
use std::collections::{HashMap, HashSet};
pub fn get_links_html<R: std::io::Read>(html_doc: R) -> anyhow::Result<HashSet<String>> {
    Ok(
        select::document::Document::from_read(encoding_rs_io::DecodeReaderBytes::new(html_doc))?
            .find(select::predicate::Name("a"))
            .filter_map(|element| element.attr("href").map(|s| s.to_string()))
            .collect::<HashSet<String>>(),
    )
}

///
/// Ouput URL graph to a writer.
///
/// E.g:
///
/// ```text
/// output_graph(&site_graph, std::io::stdout())?;
/// ```
pub fn output_graph<W: std::io::Write>(
    graph: &HashMap<Url, HashSet<Url>>,
    mut out: W,
) -> Result<()> {
    for (url, links) in graph {
        if links.is_empty() {
            continue;
        }
        out.write_all(format!("\n{} links to:", url).as_bytes())?;
        for link in links {
            out.write_all(format!("\n\t{}", link).as_bytes())?;
        }
    }
    Ok(())
}

#[derive(Debug, structopt::StructOpt)]
pub struct Cli {
    /// Url to start the crawl from.
    pub url: Url,
}
