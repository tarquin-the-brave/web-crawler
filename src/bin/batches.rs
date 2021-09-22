use anyhow::{anyhow, Result};
use reqwest::{Client, Url};
use std::collections::{HashMap, HashSet};
use web_crawler::{get_links_html, output_graph, Cli};
fn main() -> anyhow::Result<()> {
    let args = {
        use structopt::StructOpt as _;
        Cli::from_args()
    };

    let host = args
        .url
        .host_str()
        .ok_or_else(|| anyhow!("URL given has no host."))?
        .to_owned();

    let mut to_visit: HashSet<Url> = HashSet::new();
    let mut visited: HashSet<Url> = HashSet::new();
    let mut site_graph: HashMap<Url, HashSet<Url>> = HashMap::new();
    let mut broken_links: HashSet<Url> = HashSet::new();
    let client = reqwest::Client::new();

    to_visit.insert(args.url.clone());

    loop {
        let to_process = to_visit.difference(&visited).collect::<HashSet<&Url>>();
        if to_process.is_empty() {
            break;
        }

        let results = async {
            use futures::stream::StreamExt as _;
            to_process
                .into_iter()
                .map(|url| fetch_links(&client, url, &host))
                .collect::<futures::stream::FuturesUnordered<_>>()
                .collect()
                .await
        };
        let rt = tokio::runtime::Runtime::new()?;
        let results: Vec<Result<(Url, HashSet<Url>), Url>> = rt.block_on(results);
        let (good, bad): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);

        bad.into_iter().map(Result::unwrap_err).for_each(|url| {
            visited.insert(url.clone());
            broken_links.insert(url);
        });

        good.into_iter()
            .map(Result::unwrap)
            .for_each(|(url, links)| {
                visited.insert(url.clone());
                for link in &links {
                    let linkstr = link.as_str();
                    if linkstr.ends_with(".png")
                        || linkstr.ends_with(".jpeg")
                        || linkstr.ends_with(".jpg")
                        || linkstr.ends_with(".htm")
                        || linkstr.ends_with(".pdf")
                    {
                        continue;
                    }
                    to_visit.insert(link.clone());
                }
                site_graph.insert(url, links);
            });
    }

    output_graph(&site_graph, std::io::stdout())?;

    Ok(())
}

async fn fetch_links(
    client: &Client,
    url: &reqwest::Url,
    host: &str,
) -> Result<(Url, HashSet<Url>), Url> {
    fetch_links_inner(client, url, host)
        .await
        .map_err(|_| {
            println!("link {} was broken", url);
            url.clone()
        })
        .map(|links| {
            println!("found {} links from: {}", links.len(), url);
            (url.clone(), links)
        })
}

async fn fetch_links_inner(
    client: &Client,
    url: &reqwest::Url,
    site_host: &str,
) -> Result<HashSet<Url>> {
    client
        .get(url.clone())
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await
        .map_err(Into::into)
        .map(std::io::Cursor::new)
        .and_then(get_links_html)
        .map(|links| {
            links
                .into_iter()
                .filter_map(|link| {
                    if let Some(link) = Url::parse(&link).ok() {
                        link.host_str().and_then(|link_host| {
                            if link_host == site_host {
                                Some(url.clone())
                            } else {
                                None
                            }
                        })
                    } else {
                        // It might be a relative link, try to join it to the URL it is linked
                        // from.
                        url.join(&link).ok()
                    }
                })
                .collect()
        })
}
