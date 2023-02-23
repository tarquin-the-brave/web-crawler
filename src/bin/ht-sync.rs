use reqwest::Url;
use std::collections::{HashMap, HashSet};
use structopt::StructOpt;
use web_crawler::{get_links_from_string, output_graph};

/// Synchronous web crawler
#[derive(Debug, StructOpt)]
pub struct Opt {
    /// Url to start the crawl from.
    pub url: Url,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    // List of all links found
    let mut links: HashMap<Url, HashSet<Url>> = HashMap::new();

    let mut reqwest_client = reqwest::blocking::Client::builder()
        .user_agent("my_test")
        .build()?;

    get_links(&mut reqwest_client, opt.url, &mut links)?;

    output_graph(&links, std::io::stdout())?;
    Ok(())
}

/// Find links for the given URL (recursively) and add them to links
fn get_links(
    reqwest_client: &mut reqwest::blocking::Client,
    url: Url,
    links: &mut HashMap<Url, HashSet<Url>>,
) -> anyhow::Result<()> {
    let response = reqwest_client.get(url.to_string()).send()?;

    // Continue for invalid URLs
    if !response.status().is_success() {
        println!("URL {} could not be found", url);
        return Ok(());
    }

    // A string containing the page's HTML
    let page_html = response.error_for_status()?.text()?;

    let links_from_this_url = get_links_from_string(page_html, &url)?;

    // Save the links found for this URL
    links.insert(url.clone(), links_from_this_url.clone());

    for mut link in links_from_this_url {
        // We don't want to search different fragments on the same page multiple times
        link.set_fragment(None);

        // Don't look for links on external pages, or if we've already searched the page before
        if link.host() != url.host() || links.contains_key(&link) {
            continue;
        }

        get_links(reqwest_client, link.clone(), links)?;
    }
    Ok(())
}
