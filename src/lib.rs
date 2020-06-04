use reqwest::header;
use reqwest::Client;
use reqwest::RequestBuilder;
use reqwest::StatusCode;
use reqwest::Url;
use serde_json::Value;
use std::io;
use structopt::StructOpt;
use tokio::runtime::Runtime;

pub const NAME: &str = env!("CARGO_PKG_NAME");
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug, StructOpt)]
#[structopt(
name = NAME,
about = "Basic curl but automatically follows next page links"
)]
pub struct Opt {
    /// Basic auth
    #[structopt(short = "u", long = "user", name = "user:password")]
    pub user_password: Option<String>,

    /// How many values to return (determines how many pages are fetched)
    #[structopt(short = "n", long = "number")]
    pub number: Option<usize>,

    pub url: String,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn run(opt: Opt) -> Result<()> {
    let mut rt = Runtime::new()?;
    rt.block_on(async { run_async(opt).await })
}

pub async fn run_async(opt: Opt) -> Result<()> {
    let client = Client::builder().user_agent(APP_USER_AGENT).build()?;

    let start_url = Url::parse(&opt.url)?;
    let mut pager = Pager::new(&start_url);

    let values_limit = opt.number.unwrap_or(std::usize::MAX);
    let mut values_printed = 0;
    let mut next_url = Some(start_url);
    while let Some(url) = next_url {
        let request = build_request(url.clone(), &client, &opt);

        let response = request.send().await?;
        if response.status() == StatusCode::NOT_FOUND && values_printed > 0 {
            // This is not our first request, it's a next page. If that's not found, assume
            // we've reached the end.
            break;
        }

        if !response.status().is_success() {
            let status = response.status().clone();
            let body = response.text().await.unwrap_or(String::new());
            return Err(format!("Error getting {}: {}: {}", url, status, body).into());
        }

        let next_link_from_header = response
            .headers()
            .get(header::LINK)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_next_link)
            .map(|s| s.to_string());

        let body = response.json::<Value>().await?;

        if let Some(values) = body
            .get("values")
            .and_then(|v| v.as_array())
            .or_else(|| body.get("items").and_then(|v| v.as_array()))
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
            return Err(format!(r#"Could not read values from response. Expected either `{{"values": [...]}}` or `[...]`, got: {}"#, body).into());
        }

        if values_printed < values_limit {
            let from_response = next_link_from_header.or_else(|| {
                body.get("next")
                    .and_then(|o| o.as_str())
                    .map(|s| s.to_string())
            });
            next_url = pager.next(from_response.as_deref())?;
        } else {
            next_url = None;
        }
    }

    Ok(())
}

fn build_request(url: Url, client: &Client, opt: &Opt) -> RequestBuilder {
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

/// See https://tools.ietf.org/html/rfc5988#section-5
///
/// Example header:
///
///     <https://api.github.com/search/code?q=addClass+user%3Amozilla&page=2>; rel="next",
///      <https://api.github.com/search/code?q=addClass+user%3Amozilla&page=34>; rel="last""
fn parse_next_link(link_header: &str) -> Option<&str> {
    if let Some(end) = link_header.find(r#">; rel="next""#) {
        let s = &link_header[0..end];
        if let Some(start) = s.rfind("<") {
            return Some(&s[start + 1..]);
        }
    }
    return None;
}

struct Pager {
    start_url: Url,
    page: Option<usize>,
    page_param: Option<String>,
    query_params: Vec<(String, String)>,
    try_page_numbers: bool,
}

impl Pager {
    fn new(start_url: &Url) -> Self {
        let mut page_param = None;
        let mut query_params = Vec::new();
        for (key, value) in start_url.query_pairs() {
            if &key == "page" {
                page_param = Some(value.to_string());
            } else {
                query_params.push((key.to_string(), value.to_string()));
            }
        }
        Pager {
            start_url: start_url.clone(),
            page: None,
            page_param,
            query_params,
            try_page_numbers: true,
        }
    }

    fn next(&mut self, next_url_from_response: Option<&str>) -> Result<Option<Url>> {
        // If there's a "next" URL, use that from now on. Otherwise try `page=N` query param.
        if let Some(next) = next_url_from_response {
            self.try_page_numbers = false;
            Ok(Some(self.start_url.join(next)?))
        } else if self.try_page_numbers {
            let mut page_number = if let Some(page_number) = self.page {
                page_number
            } else {
                // Ok, first time we're trying to page.
                // If the start URL had a page param, try to parse it as a number.
                if let Some(param) = &self.page_param {
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
            self.page = Some(page_number);

            let mut url = self.start_url.clone();
            url.query_pairs_mut()
                .clear()
                .extend_pairs(&self.query_params)
                .append_pair("page", &format!("{}", page_number));
            Ok(Some(url))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_next_link() {
        assert_eq!(parse_next_link(""), None);
        assert_eq!(parse_next_link("<https://api.github.com/search/code?q=addClass+user%3Amozilla&page=2>; rel=\"next\""),
                   Some("https://api.github.com/search/code?q=addClass+user%3Amozilla&page=2"));
        assert_eq!(parse_next_link("<https://api.github.com/search/code?q=addClass+user%3Amozilla&page=1>; rel=\"prev\", <https://api.github.com/search/code?q=addClass+user%3Amozilla&page=3>; rel=\"next\""),
                   Some("https://api.github.com/search/code?q=addClass+user%3Amozilla&page=3"));
    }
}
