use anyhow::{anyhow, Result};
use reqwest::Url;
use std::collections::HashSet;
use tokio::{
    sync::mpsc::{
        unbounded_channel as channel, UnboundedReceiver as Receiver, UnboundedSender as Sender,
    },
    task::spawn,
};
use web_crawler::{get_links_html, output_graph, Cli, UrlGraph};

#[derive(Debug)]
enum LinksMsg {
    Failed(Url),
    Links(Url, HashSet<Url>),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Cli = structopt::StructOpt::from_args();

    let host = args
        .url
        .host_str()
        .ok_or_else(|| anyhow!("URL given has no host."))?
        .to_owned();

    let (url_tx, url_rx) = channel::<Url>();
    let (links_tx, links_rx) = channel::<LinksMsg>();
    let (done_tx, mut done_rx) = channel::<UrlGraph>();

    url_tx.send(args.url)?;
    spawn(fetcher(host, url_rx, links_tx));
    spawn(graph_builder(links_rx, url_tx, done_tx));

    let url_graph = done_rx.recv().await.unwrap();

    output_graph(&url_graph, std::io::stdout())?;
    Ok(())
}

async fn fetcher(host: String, mut url_rx: Receiver<Url>, links_tx: Sender<LinksMsg>) {
    loop {
        match url_rx.recv().await {
            None => return,
            Some(url) => {
                spawn(get_links(host.clone(), url, links_tx.clone()));
            }
        };
    }
}

async fn graph_builder(
    mut links_rx: Receiver<LinksMsg>,
    url_tx: Sender<Url>,
    done_tx: Sender<UrlGraph>,
) -> Result<()> {
    let mut url_graph = UrlGraph::new();
    let mut in_flight = HashSet::<Url>::new();
    loop {
        match links_rx
            .recv()
            .await
            .ok_or_else(|| anyhow!("links senders all dropped"))?
        {
            LinksMsg::Failed(url) => {
                in_flight.remove(&url);
            }
            LinksMsg::Links(url, links) => {
                in_flight.remove(&url);
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
                    let mut link_scrubed = link.clone();
                    link_scrubed.set_fragment(None);
                    link_scrubed.set_query(None);

                    if link_scrubed != url && !url_graph.contains_key(&link_scrubed) {
                        url_tx.send(link_scrubed.clone())?;
                        in_flight.insert(link_scrubed);
                    }
                }
                url_graph.insert(url, links);
            }
        }

        if in_flight.is_empty() {
            break;
        }
    }

    done_tx.send(url_graph)?;

    Ok(())
}

async fn get_links(site_host: String, url: Url, links_tx: Sender<LinksMsg>) -> Result<()> {
    let msg = if let Ok(links) = get_links_inner(&site_host, &url).await {
        println!("Got {} links from {}", links.len(), url);
        LinksMsg::Links(url, links)
    } else {
        println!("Broken link {}", url);
        LinksMsg::Failed(url)
    };

    links_tx.send(msg)?;

    Ok(())
}

async fn get_links_inner(site_host: &str, url: &Url) -> Result<HashSet<Url>> {
    reqwest::get(url.clone())
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
                                Some(link.clone())
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
