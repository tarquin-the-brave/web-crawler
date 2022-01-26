use reqwest::Url;
use std::collections::{HashMap, HashSet};
use structopt::StructOpt;
use tokio::sync::mpsc::{self, UnboundedSender};
use web_crawler::{get_links_from_string, output_graph};

/// Synchronous web crawler
#[derive(Debug, StructOpt)]
pub struct Opt {
    /// Url to start the crawl from.
    pub url: Url,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    // List of all links found
    let mut links = HashMap::<Url, HashSet<Url>>::new();

    let reqwest_client = reqwest::Client::builder().user_agent("my_test").build()?;

    let (tx, mut rx) = mpsc::unbounded_channel::<(Url, HashSet<Url>)>();

    let tx_clone = tx.clone();
    let mut reqwest_client_clone = reqwest_client.clone();

    let mut outstanding_searches: u128 = 1;

    tokio::spawn(async move {
        get_links(&mut reqwest_client_clone, opt.url, tx_clone)
            .await
            .unwrap();
    });

    while let Some((url, links_from_this_url)) = rx.recv().await {
        outstanding_searches -= 1;
        links.insert(url.clone(), links_from_this_url.clone());

        links_from_this_url.iter().for_each(|link| {
            let mut reqwest_client = reqwest_client.clone();
            let tx_clone = tx.clone();
            let mut link = link.clone();

            link.set_fragment(None);

            // Don't look for links on external pages, or if we've already searched the page before
            if link.host() != url.host() || links.contains_key(&link) {
                return;
            }

            // Insert this link into links to try and ensure this is the only thread which will be started searching this URL
            // Adding this line caused a reduction in time taken crawling https://serde.rs from ~20s to ~1s
            links.insert(link.clone(), HashSet::new());

            outstanding_searches += 1;
            tokio::spawn(async move {
                get_links(&mut reqwest_client, link.clone(), tx_clone)
                    .await
                    .unwrap();
            });
        });

        if outstanding_searches == 0 {
            break;
        }
    }

    output_graph(&links, std::io::stdout())?;
    Ok(())
}

/// Find links for the given URL (recursively) and add them to links
async fn get_links(
    reqwest_client: &mut reqwest::Client,
    url: Url,
    tx: UnboundedSender<(Url, HashSet<Url>)>,
) -> anyhow::Result<()> {
    let response = reqwest_client.get(url.to_string()).send().await?;

    // Continue for invalid URLs
    if !response.status().is_success() {
        tx.send((url.clone(), HashSet::new()))?;
        return Ok(());
    }

    // A string containing the page's HTML
    let page_html = response.error_for_status()?.text().await?;

    let links_from_this_url = get_links_from_string(page_html, &url)?;

    tx.send((url.clone(), links_from_this_url))?;

    Ok(())
}
