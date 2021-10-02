use anyhow::Result;
use reqwest::Url;
use std::collections::{HashMap, HashSet};

pub type UrlGraph = HashMap<Url, HashSet<Url>>;

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
    let pages = graph.len();
    let unique_urls = graph
        .values()
        .cloned()
        .fold(HashSet::new(), |acc, x| acc.union(&x).cloned().collect())
        .len();
    let total_links: usize = graph.values().map(|v| v.len()).sum();
    out.write_all(format!("\nFound {} unique pages:", pages).as_bytes())?;
    out.write_all(format!("\nFound {} unique URLs:", unique_urls).as_bytes())?;
    out.write_all(format!("\nFound {} total links:", total_links).as_bytes())?;

    for (url, links) in graph {
        out.write_all(format!("\n{} links to:", url).as_bytes())?;
        if links.is_empty() {
            out.write_all("\n\tNothing".as_bytes())?;
        }
        for link in links {
            out.write_all(format!("\n\t{}", link).as_bytes())?;
        }
    }
    Ok(())
}

///
/// Basic CLI
///
/// Usage:
///
/// ```
/// main() {
///     let args = {
///         use structopt::StructOpt as _;
///         Cli::from_args()
///     };
///     println!("arg given: {}", args.url);
/// }
/// ```
#[derive(Debug, structopt::StructOpt)]
pub struct Cli {
    /// Url to start the crawl from.
    pub url: Url,
}
