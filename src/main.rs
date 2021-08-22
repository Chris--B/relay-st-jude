use color_eyre::Report;
use serde::Deserialize;
use serde_json::json;

#[allow(unused_imports)]
use tracing::{info, warn};

use std::fmt;

#[derive(Deserialize, Clone, Debug)]
struct ApiResponse {
    #[serde(default)]
    errors: Vec<ApiError>,
}

#[derive(Deserialize, Clone, Copy, Debug)]
struct ApiLocation {
    line: usize,
    column: usize,
}

#[derive(Deserialize, Clone, Debug)]
struct ApiError {
    message: String,
    #[serde(default)]
    locations: Vec<ApiLocation>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.locations.is_empty() {
            write!(f, "~ {}", self.message)
        } else {
            let loc = self.locations[0];
            write!(f, "~:{}:{} {}", loc.line, loc.column, self.message)
        }
    }
}

#[tracing::instrument]
fn query_campaign_status() -> Result<ApiResponse, Report> {
    const API_URL: &str = "https://api.tiltify.com";

    let graph_ql_query = json!({
        "operationName": "get_campaign_by_vanity_and_slug",
        "variables": {
            "vanity": "@relay-fm",
            "slug": "relay-st-jude-21"
        },
        "query": indoc::indoc!(r#"query get_campaign_by_vanity_and_slug($vanity: String, $slug: String) {
            campaign(vanity: $vanity, slug: $slug) {
                id
                name
                slug
                status
                originalGoal {
                    value
                    currency
                }
                team {
                    name
                }
                description
                totalAmountRaised {
                    currency
                    value
                }
                goal {
                    currency
                    value
                }
                avatar {
                    alt
                    height
                    width
                    src
                }
                milestones {
                    id
                    name
                    amount {
                        value
                        currency
                    }
                }
            }
        }"#)
    });
    // let request = serde_json::to_string(&graph_ql_query)?;
    let request = serde_json::to_string_pretty(&graph_ql_query)?;
    std::fs::write("./target/request.json", &request)?;

    let res = reqwest::blocking::Client::new()
        .post(API_URL)
        .body(request.clone())
        .send()?;

    let res_json = res.text()?;
    std::fs::write("./target/response.json", &res_json)?;

    let res: ApiResponse = serde_json::from_str(&res_json)?;
    if res.errors.is_empty() {
        Ok(res)
    } else {
        // Iunno if we'll ever get multiple errors, so don't worry about combining them yet
        if res.errors.len() > 1 {
            warn!(?res, "Expected 0 or 1 failures but got 2?",);
        }

        // Print the section it's talking about
        {
            let ApiLocation { line, column } = res.errors[0].locations[0];
            let mut lines: Vec<_> = request.lines().collect();
            // Lines are 1-indexed, so insert a dummy line at the start. This way we don't have to translate accesses.
            lines.insert(0, "<internal filler line>");

            let arm: String = std::iter::repeat('~').take(column as usize - 1).collect();

            // One before, but not our filler line
            #[allow(clippy::int_plus_one)]
            if line - 1 >= 1 {
                println!("{:>3}: {}", line - 1, lines[line - 1]);
            }

            // Main line
            println!("{:>3}: {}", line, &lines[line]);
            println!("{:>3}  {}^", "", arm);

            // One after
            if line + 1 < lines.len() {
                println!("{:>3}: {}", line + 1, lines[line + 1]);
            }
        }

        // And then return
        Err(Report::msg(res.errors[0].clone()))
    }
}

fn main() -> Result<(), Report> {
    setup()?;

    let status = query_campaign_status()?;
    println!("status: {:#?}", status);

    Ok(())
}

fn setup() -> Result<(), Report> {
    use std::env;
    use tracing_subscriber::EnvFilter;

    if env::var("RUST_LIB_BACKTRACE").is_err() {
        env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install()?;

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn example_response() {
        const RESPONSE: &str = include_str!("example-response.json");

        let _response: ApiResponse = serde_json::from_str(RESPONSE).unwrap();
    }
}
