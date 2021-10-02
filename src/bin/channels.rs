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
enum Message<T> {
    Finished,
    Msg(T),
}

fn msg<T>(contents: T) -> Message<T> {
    Message::Msg(contents)
}

type UrlMsg = Message<Url>;
type LinksMsg = Message<(Url, HashSet<Url>)>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Cli = structopt::StructOpt::from_args();

    let host = args
        .url
        .host_str()
        .ok_or_else(|| anyhow!("URL given has no host."))?
        .to_owned();

    let (url_tx, url_rx) = channel::<UrlMsg>();
    let (links_tx, links_rx) = channel::<LinksMsg>();
    let (done_tx, mut done_rx) = channel::<UrlGraph>();

    url_tx.send(msg(args.url))?;
    spawn(fetcher(host, url_rx, links_tx));
    spawn(graph_builder(links_rx, url_tx, done_tx));

    let url_graph = done_rx.recv().await.unwrap();

    output_graph(&url_graph, std::io::stdout())?;
    Ok(())
}

async fn fetcher(host: String, mut url_rx: Receiver<UrlMsg>, links_tx: Sender<LinksMsg>) {
    loop {
        match url_rx.recv().await {
            None => return,
            Some(UrlMsg::Finished) => links_tx.send(LinksMsg::Finished).unwrap(),
            Some(UrlMsg::Msg(url)) => {
                spawn(get_links(host.clone(), url, links_tx.clone()));
            }
        };
    }
}

async fn graph_builder(
    mut links_rx: Receiver<LinksMsg>,
    url_tx: Sender<UrlMsg>,
    done_tx: Sender<UrlGraph>,
) {
    let mut maybe_finished = false;
    let mut url_graph = UrlGraph::new();
    loop {
        match links_rx.recv().await {
            None => return,
            Some(LinksMsg::Finished) => {
                if maybe_finished {
                    break;
                } else {
                    continue;
                }
            }
            Some(LinksMsg::Msg((url, links))) => {
                let mut new_urls = false;
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
                        new_urls = true;
                        url_tx.send(msg(link_scrubed)).unwrap();
                    }
                }
                url_graph.insert(url, links);

                if !new_urls {
                    url_tx.send(UrlMsg::Finished).unwrap();
                    maybe_finished = true;
                }
            }
        }
    }

    done_tx.send(url_graph).unwrap();
}

async fn get_links(site_host: String, url: Url, links_tx: Sender<LinksMsg>) -> Result<()> {
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
                .collect::<HashSet<Url>>()
        })
        .and_then(|links| links_tx.send(msg((url, links))).map_err(Into::into))
}
