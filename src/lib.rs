use reqwest::Url;
pub fn get_links_html<R: std::io::Read>(html_doc: R) -> anyhow::Result<Vec<Url>> {
    Ok(
        select::document::Document::from_read(encoding_rs_io::DecodeReaderBytes::new(html_doc))?
            .find(select::predicate::Name("a"))
            .filter_map(|element| Url::parse(element.attr("href").unwrap()).ok())
            .collect::<Vec<Url>>(),
    )
}

pub fn output_graph(graph: &std::collections::HashMap<Url, Vec<Url>>) {
    for (url, links) in graph {
        if links.is_empty() {
            continue;
        }
        println!("{} links to:", url);
        for link in links {
            println!("\t{}", link);
        }
    }
}

#[derive(Debug, structopt::StructOpt)]
pub struct Cli {
    /// Url to start the crawl from.
    pub url: Url,
}
