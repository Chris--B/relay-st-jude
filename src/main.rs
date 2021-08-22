use color_eyre::Report;
use serde::{de, Deserialize, Deserializer};
use serde_json::json;

#[allow(unused_imports)]
use tracing::{info, warn};

use std::fmt;

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct Milestone {
    name: String,
    amount: CurrencyAmount,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct Campaign {
    name: String,
    description: String,
    slug: String,
    status: String,

    goal: CurrencyAmount,
    #[serde(rename = "totalAmountRaised")]
    total_amount_raised: CurrencyAmount,

    #[serde(default)]
    milestones: Vec<Milestone>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct CurrencyAmount {
    /// Currency our amount is in. Often just USD
    currency: String,

    /// Amount that this represents
    #[serde(deserialize_with = "deserialize_f64_from_str")]
    value: f64,
}

impl CurrencyAmount {
    #[allow(dead_code)]
    fn usd(value: f64) -> Self {
        Self {
            currency: "USD".to_string(),
            value,
        }
    }
}

impl From<CurrencyAmount> for f64 {
    fn from(amount: CurrencyAmount) -> f64 {
        // :shrug:
        assert_eq!(amount.currency, "USD");
        amount.value
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct Data {
    campaign: Campaign,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ApiResponse {
    data: Option<Data>,

    #[serde(default)]
    errors: Vec<ApiError>,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
struct ApiLocation {
    line: usize,
    column: usize,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
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

fn deserialize_f64_from_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(de::Error::custom)
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

    let request = serde_json::to_string_pretty(&graph_ql_query)?;
    std::fs::write("./target/request.json", &request)?;

    let response = ureq::post(API_URL).send_json(graph_ql_query)?;

    let res_json = response.into_string()?;
    std::fs::write("./target/response.json", &res_json)?;

    let res: ApiResponse = serde_json::from_str(&res_json)?;
    if res.errors.is_empty() {
        Ok(res)
    } else {
        // Iunno if we'll ever get multiple errors, so don't worry about combining them yet
        if res.errors.len() > 1 {
            warn!(?res, "Expected 0 or 1 failures but got 2?",);
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

    /// Verify that our saved JSON from the API matches our serde model
    #[test]
    fn example_response() {
        const DESCRIPTION: &str = "Every September, the Relay FM community of podcasters and listeners rallies together to support the lifesaving mission of St. Jude Children’s Research Hospital during Childhood Cancer Awareness Month. Throughout the month, Relay FM will introduce ways to support St. Jude through entertaining donation challenges and other mini-fundraising events that will culminate in the second annual Relay for St. Jude Podcastathon on September 17th beginning at 12pm Eastern at twitch.tv/relayfm.";
        const RESPONSE: &str = include_str!("example-response.json");

        let expected = ApiResponse {
            data: Some(Data {
                campaign: Campaign {
                    name: "Relay FM for St. Jude 2021".to_string(),
                    description: DESCRIPTION.to_string(),
                    slug: "relay-st-jude-21".to_string(),
                    status: "published".to_string(),

                    goal: CurrencyAmount::usd(333_333.33),
                    total_amount_raised: CurrencyAmount::usd(22663.40),

                    milestones: vec![
                        Milestone {
                            amount: CurrencyAmount::usd(75000.00),
                            name: "Stephen & Myke go to space via KSP".to_string(),
                        },
                        Milestone {
                            amount: CurrencyAmount::usd(55000.00),
                            name: "Stephen dissembles his NeXTCube on stream".to_string(),
                        },
                        Milestone {
                            amount: CurrencyAmount::usd(20000.00),
                            name: "Myke and Stephen attempt Flight Simulator again".to_string(),
                        },
                        Milestone {
                            amount: CurrencyAmount::usd(196060.44),
                            name: "$1 million raised in 3 years!".to_string(),
                        },
                    ],
                },
            }),

            errors: vec![],
        };

        assert_eq!(expected, serde_json::from_str(RESPONSE).unwrap());
    }
}
