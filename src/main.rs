use reqwest::Client;
use reqwest::RequestBuilder;
use reqwest::StatusCode;
use reqwest::Url;
use serde_json::Value;
use std::{io, process};
use structopt::StructOpt;
use tokio::runtime::Runtime;

const NAME: &str = "concurl";

#[derive(Debug, StructOpt)]
#[structopt(
    name = NAME,
    about = "Basic curl but automatically follows next page links"
)]
struct Opt {
    /// Basic auth
    #[structopt(short = "u", long = "user", name = "user:password")]
    user_password: Option<String>,

    /// How many values to return (determines how many pages are fetched)
    #[structopt(short = "n", long = "number")]
    number: Option<usize>,

    url: String,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    let opt = Opt::from_args();
    match run(opt) {
        Err(err) => {
            eprintln!("{}: {}", NAME, err);
            process::exit(1);
        }
        Ok(_) => {
            process::exit(0);
        }
    }
}

fn run(opt: Opt) -> Result<()> {
    let mut rt = Runtime::new()?;
    rt.block_on(async { run_async(opt).await })
}

async fn run_async(opt: Opt) -> Result<()> {
    let client = Client::new();

    let start_url = Url::parse(&opt.url)?;
    let (page_param, query_pairs) = parse_page_query(&start_url);
    let mut page = None;
    let mut try_page_numbers = true;

    let values_limit = opt.number.unwrap_or(std::usize::MAX);
    let mut values_printed = 0;
    let mut next_url = Some(start_url);
    while let Some(mut url) = next_url {
        let request = request(url.clone(), &client, &opt);

        let response = request.send().await?;
        if response.status() == StatusCode::NOT_FOUND && values_printed > 0 {
            // This is not our first request, it's a next page. If that's not found, assume
            // we've reached the end.
            break;
        }

        if !response.status().is_success() {
            let status = response.status().clone();
            let body = response.text().await?;
            return Err(format!("Error getting {}: {}: {}", url, status, body).into());
        }

        let body = response.json::<Value>().await?;

        if let Some(values) = body
            .get("values")
            .and_then(|v| v.as_array())
            .or_else(|| body.as_array())
        {
            if values.is_empty() {
                // No more values returned, assume we've reached the end.
                break;
            }

            for value in values {
                serde_json::to_writer(io::stdout(), value)?;
                println!();
                values_printed += 1;

                if values_printed == values_limit {
                    break;
                }
            }
        } else {
            return Err(format!("Could not read values from response. Expected either `{"values": [...]}` or `[...]`, got: {}", body).into());
        }

        next_url = None;
        if values_printed < values_limit {
            // If there's a "next" URL, use that. Otherwise try `page=N` query param
            if let Some(next) = body.get("next").and_then(|o| o.as_str()) {
                next_url = Some(Url::parse(next)?);
                try_page_numbers = false;
            } else if try_page_numbers {
                let mut page_number = if let Some(page_number) = page {
                    page_number
                } else {
                    // Ok, first time we're trying to page.
                    // If the start URL had a page param, try to parse it as a number.
                    if let Some(param) = &page_param {
                        param.parse().map_err(|e| {
                            format!(
                                "Page query param '{}' could not be parsed as a number: {}",
                                param, e
                            )
                        })?
                    } else {
                        // Otherwise assume we were at page 1
                        1
                    }
                };

                page_number += 1;

                url.query_pairs_mut()
                    .clear()
                    .extend_pairs(&query_pairs)
                    .append_pair("page", &format!("{}", page_number));
                next_url = Some(url);
                page = Some(page_number);
            }
        }
    }

    Ok(())
}

fn parse_page_query(start_url: &Url) -> (Option<String>, Vec<(String, String)>) {
    let mut page = None;
    let mut query_pairs = Vec::new();
    for (key, value) in start_url.query_pairs() {
        if &key == "page" {
            page = Some(value.to_string());
        } else {
            query_pairs.push((key.to_string(), value.to_string()));
        }
    }
    (page, query_pairs)
}

fn request(url: Url, client: &Client, opt: &Opt) -> RequestBuilder {
    let mut request = client.get(url);
    if let Some(user_password) = &opt.user_password {
        let parts: Vec<_> = user_password.splitn(2, ":").collect();
        if parts.len() == 1 {
            request = request.basic_auth(&parts[0], Option::<&str>::None);
        } else {
            request = request.basic_auth(&parts[0], Some(&parts[1]));
        }
    }
    request
}
