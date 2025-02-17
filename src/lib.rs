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

pub fn get_links_from_string(html_string: String, page_url: &Url) -> anyhow::Result<HashSet<Url>> {
    // The links on this page as strings
    let raw_links_from_this_url = get_links_html(html_string.as_bytes())?;

    // The links as URL objects
    let mut cleansed_links_from_this_url: HashSet<Url> = HashSet::new();

    // Get the URL object from the raw link, handling relative and absolute URLs correctly
    for link in raw_links_from_this_url {
        if let Ok(absolute_url) = Url::parse(&link) {
            // This is an absolute link
            cleansed_links_from_this_url.insert(absolute_url);
        } else {
            // This isn't an absolute link, so assume it's relative and join it to the current page's URL
            cleansed_links_from_this_url.insert(page_url.join(&link)?);
        };
    }
    Ok(cleansed_links_from_this_url)
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

    for (url, links) in graph {
        out.write_all(format!("\n{} links to:\n", url).as_bytes())?;
        if links.is_empty() {
            out.write_all("\tNothing".as_bytes())?;
        }
        for link in links {
            out.write_all(format!("\t{}\n", link).as_bytes())?;
        }
    }

    out.write_all(format!("\nFound {} unique pages", pages).as_bytes())?;
    out.write_all(format!("\nFound {} unique URLs", unique_urls).as_bytes())?;
    out.write_all(format!("\nFound {} total links", total_links).as_bytes())?;

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
