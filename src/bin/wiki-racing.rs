use anyhow::{anyhow, Result};
use reqwest::Url;
use std::collections::HashSet;
use tokio::{
    sync::mpsc::{
        unbounded_channel as channel, UnboundedReceiver as Receiver, UnboundedSender as Sender,
    },
    task::{spawn, spawn_blocking},
};
use web_crawler::{get_links_html, Cli};

const WIKIPEDIA: &str = "en.wikipedia.org";

struct LinksMsg(Vec<Url>, HashSet<Url>);

struct DoneMsg(Vec<String>, usize);

type UrlMsg = Vec<Url>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Cli = structopt::StructOpt::from_args();

    let start = Url::parse(&format!("https://{}/wiki/{}", WIKIPEDIA, args.start))?;
    let end = Url::parse(&format!("https://{}/wiki/{}", WIKIPEDIA, args.end))?;

    let (url_tx, url_rx) = channel::<UrlMsg>();
    let (links_tx, links_rx) = channel::<LinksMsg>();
    let (done_tx, mut done_rx) = channel::<DoneMsg>();

    url_tx.send(vec![start.clone()])?;
    let fetcher = spawn(fetcher(url_rx, links_tx));
    let trail_builder = spawn(trail_builder(links_rx, url_tx, done_tx, start, end));

    let done_msg = done_rx.recv().await.unwrap();

    fetcher.abort();
    trail_builder.abort();

    spawn_blocking(|| output_route(done_msg)).await?;
    Ok(())
}

fn output_route(done_msg: DoneMsg) {
    let DoneMsg(trail, visits) = done_msg;
    let steps = trail.len() - 1;
    println!("\nCan be done in {steps} steps\n");
    let example = itertools::join(trail, " -> ");
    println!("{example}");
    println!("\n{visits} pages were visited\n");
}

async fn fetcher(mut url_rx: Receiver<UrlMsg>, links_tx: Sender<LinksMsg>) {
    // Client contains an `Arc` internally.  No need to wrap it in one.
    let client = reqwest::Client::new();
    loop {
        match url_rx.recv().await {
            None => return,
            Some(url) => {
                spawn(get_links(client.clone(), url, links_tx.clone()));
            }
        };
    }
}

async fn trail_builder(
    mut links_rx: Receiver<LinksMsg>,
    url_tx: Sender<UrlMsg>,
    done_tx: Sender<DoneMsg>,
    start: Url,
    end: Url,
) -> Result<()> {
    let mut visits = 0;
    let mut requested_links = HashSet::new();
    requested_links.insert(start);
    loop {
        let LinksMsg(mut url_trail, links) = links_rx
            .recv()
            .await
            .ok_or_else(|| anyhow!("links senders all dropped"))?;

        visits += 1;

        for link in &links {
            let link_path = link.path();
            if link_path == end.path() {
                // We've found the end destination!
                url_trail.push(link.clone());
                let route = url_trail
                    .iter()
                    .map(|url| url.path_segments().unwrap().last().unwrap().to_string())
                    .collect();
                done_tx.send(DoneMsg(route, visits))?;
                return Ok(());
            }

            if link_path.ends_with(".png")
                || link_path.ends_with(".jpeg")
                || link_path.ends_with(".jpg")
                || link_path.ends_with(".htm")
                || link_path.ends_with(".pdf")
                || link_path.ends_with(".php")
            {
                continue;
            }

            let mut scrubbed_link = link.clone();
            scrubbed_link.set_fragment(None);
            scrubbed_link.set_query(None);

            if requested_links.insert(scrubbed_link.clone()) {
                let mut new_url_trail = url_trail.clone();
                new_url_trail.push(scrubbed_link);
                url_tx.send(new_url_trail)?;
            }
        }
    }
}

async fn get_links(
    client: reqwest::Client,
    url_trail: Vec<Url>,
    links_tx: Sender<LinksMsg>,
) -> Result<()> {
    if let Ok(links) = get_links_inner(&client, &url_trail[url_trail.len() - 1]).await {
        links_tx.send(LinksMsg(url_trail, links))?;
    };

    Ok(())
}

async fn get_links_inner(client: &reqwest::Client, url: &Url) -> Result<HashSet<Url>> {
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
                    if let Ok(link) = Url::parse(&link) {
                        link.host_str().and_then(|link_host| {
                            // To consider a link it needs to be within wikipedia and
                            // a distinct page to the URL searched.
                            if link_host == WIKIPEDIA && link.path() != url.path() {
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
