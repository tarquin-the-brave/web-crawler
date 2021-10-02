use anyhow::Result;
use reqwest::Url;
use std::collections::HashSet;
use std::str::FromStr;
use structopt::StructOpt;
use web_crawler::*;

fn main() -> anyhow::Result<()> {
    let cli = Cli::from_args();
    let mut url_graph = UrlGraph::new();
    recursive_search(cli.url, &mut url_graph)?;
    output_graph(&url_graph, std::io::stdout())?;
    Ok(())
}

fn recursive_search(url: Url, url_graph: &mut UrlGraph) -> Result<()> {
    // Get the URLs
    let html_doc = reqwest::blocking::get(url.clone())?.text()?;
    let links = get_links_html(html_doc.as_bytes())?;

    // Parse the Strings into URLs and pray
    let links: HashSet<Url> = links
        .into_iter()
        // Try to parse as URL
        // If that fails assume that it's a relative URL
        // and join to the parent
        .map(|x| match Url::from_str(&x) {
            Ok(x) => x,
            Err(_) => url.join(&x).unwrap(),
        })
        .filter(|x| x.domain() == url.domain())
        .collect();

    // Add links to graph
    url_graph.insert(url.clone(), links.clone());

    // Queries and fragments don't resolve to different pages
    // Remove queries and fragments
    let links: HashSet<Url> = links
        .into_iter()
        .map(|mut x| {
            x.set_query(None);
            x.set_fragment(None);
            x
        })
        .collect();

    // Filter out any links we've already seen
    let links: HashSet<Url> = links
        .into_iter()
        .filter(|x| !url_graph.contains_key(x))
        .collect();

    // Add new links to the graph
    for x in links {
        recursive_search(x, url_graph).unwrap();
    }

    Ok(())
}
