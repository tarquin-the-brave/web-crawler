use async_recursion::async_recursion;
use futures::stream::{self, StreamExt};
use reqwest::Url;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;
use structopt::StructOpt;
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
    let links_mutex = Arc::new(Mutex::new(HashMap::<Url, HashSet<Url>>::new()));

    let mut reqwest_client = reqwest::Client::builder().user_agent("my_test").build()?;

    get_links(&mut reqwest_client, opt.url, links_mutex.clone()).await?;

    output_graph(&*links_mutex.lock().unwrap(), std::io::stdout())?;
    Ok(())
}

/// Find links for the given URL (recursively) and add them to links
#[async_recursion]
async fn get_links(
    reqwest_client: &mut reqwest::Client,
    url: Url,
    links_mutex: Arc<Mutex<HashMap<Url, HashSet<Url>>>>,
) -> anyhow::Result<()> {
    let response = reqwest_client.get(url.to_string()).send().await?;

    // Continue for invalid URLs
    if !response.status().is_success() {
        println!("URL {} could not be found", url);
        return Ok(());
    }

    // A string containing the page's HTML
    let page_html = response.error_for_status()?.text().await?;

    let links_from_this_url = get_links_from_string(page_html, &url)?;

    {
        let links = &mut *links_mutex.lock().unwrap();

        // Save the links found for this URL
        links.insert(url.clone(), links_from_this_url.clone());
    }

    stream::iter(links_from_this_url)
        .for_each_concurrent(None, |mut link| {
            // Clone everything which we need for each iteration so that it can be re-used
            let url = url.clone();
            let links_mutex = links_mutex.clone();
            let mut reqwest_client = reqwest_client.clone();

            async move {
                link.set_fragment(None);
                {
                    let links = &mut *links_mutex.lock().unwrap();

                    // Don't look for links on external pages, or if we've already searched the page before
                    if link.host() != url.host() || links.contains_key(&link) {
                        return;
                    }

                    // Insert this link into links to try and ensure this is the only thread which will be started searching this URL
                    // Adding this line caused a reduction in time taken crawling https://serde.rs from ~20s to ~1s
                    links.insert(link.clone(), HashSet::new());
                }
                get_links(&mut reqwest_client, link.clone(), links_mutex.clone())
                    .await
                    .unwrap();
            }
        })
        .await;
    Ok(())
}
