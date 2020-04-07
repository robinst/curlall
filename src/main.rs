use serde_json::Value;
use structopt::StructOpt;
use std::{fmt, io};
use reqwest::Url;
use reqwest::StatusCode;
use reqwest::RequestBuilder

#[derive(Debug, StructOpt)]
#[structopt(name = "concurl", about = "Basic curl but automatically follows next page links")]
struct Opt {
    /// Basic auth
    #[structopt(short = "u", long = "user", name = "user:password")]
    user_password: Option<String>,

    /// TODO
    #[structopt(short = "p", long = "path")]
    path: Vec<String>,

    /// How many values to return (determines how many pages are fetched)
    #[structopt(short = "n", long = "number")]
    number: Option<usize>,

    url: String,
}

#[derive(Debug)]
struct ResponseError {
    body: String,
}

impl ResponseError {
    fn new(body: String) -> Self {
        Self {
            body,
        }
    }
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.body)
    }
}

impl std::error::Error for ResponseError {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let client = reqwest::Client::new();

    let start_url = Url::parse(&opt.url)?;
    let mut query_pairs = Vec::new();
    let mut page = 1;
    for (key, value) in start_url.query_pairs() {
        if &key == "page" {
            page = value.parse()?;
        } else {
            query_pairs.push((key.to_string(), value.to_string()));
        }
    }

    let values_limit = opt.number.unwrap_or(std::usize::MAX);
    let mut values_printed = 0;
    let mut next_url = Some(start_url);
    while let Some(mut url) = next_url {
        let request = request_options(client.get(url.clone()), &opt);

        let response = request.send().await?;
        if response.status() == StatusCode::NOT_FOUND && values_printed > 0 {
            // This is not our first request, it's a next page. If that's not found, assume
            // we've reached the end.
            break;
        }

        if !response.status().is_success() {
            let body = response.text().await?;
            return Err(ResponseError::new(body).into());
        }

        let body = response
            .json::<Value>()
            .await?;

        if let Some(values) = body.get("values").and_then(|v| v.as_array()) {
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
        }
        next_url = None;
        if values_printed < values_limit {
            page += 1;
            // If there's a "next" URL, use that. Otherwise try `page=N` query param
            if let Some(next) = body.get("next").and_then(|o| o.as_str()) {
                next_url = Some(Url::parse(next)?);
            } else {
                url.query_pairs_mut().clear().extend_pairs(&query_pairs).append_pair("page", &format!("{}", page));
                next_url = Some(url);
            }
        }
    }

    Ok(())
}

fn request_options(request: RequestBuilder, opt: &Opt) -> RequestBuilder {
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
