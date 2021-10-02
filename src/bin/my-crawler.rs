use anyhow::Result;
use futures::future::join_all;
use futures::future::{BoxFuture, FutureExt};
use reqwest::Url;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use tokio;
use web_crawler::*;

type SendableUrlGraph = Arc<Mutex<UrlGraph>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::from_args();
    let url_graph = Arc::new(Mutex::new(UrlGraph::new()));
    recursive_search(cli.url, url_graph.clone()).await?;
    output_graph(&url_graph.lock().expect("Posioned Lock"), std::io::stdout())?;
    Ok(())
}

fn recursive_search(url: Url, url_graph: SendableUrlGraph) -> BoxFuture<'static, Result<()>> {
    async move {
        // Get the URLs
        let html_doc = reqwest::get(url.clone()).await?.text().await?;
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
        url_graph
            .lock()
            .expect("Poisoned lock")
            .insert(url.clone(), links.clone());

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
            .filter(|x| !url_graph.lock().expect("Poisoned lock").contains_key(x))
            .collect();

        // Add new links to the graph
        let mut child_searches = Vec::new();
        for x in links {
            child_searches.push(recursive_search(x, url_graph.clone()));
        }
        join_all(child_searches).await;

        Ok(())
    }
    .boxed()
}
