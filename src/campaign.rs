use color_eyre::Report;
use serde::{de, Deserialize, Deserializer};
use serde_json::json;

use std::fmt;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Milestone {
    pub name: String,
    pub amount: CurrencyAmount,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Campaign {
    pub name: String,
    pub description: String,
    pub slug: String,
    pub status: String,

    pub goal: CurrencyAmount,
    #[serde(rename = "totalAmountRaised")]
    pub total_amount_raised: CurrencyAmount,

    #[serde(default)]
    pub milestones: Vec<Milestone>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct CurrencyAmount {
    /// Currency our amount is in. Often just USD
    pub currency: String,

    /// Amount that this represents
    #[serde(deserialize_with = "deserialize_f64_from_str")]
    pub value: f64,
}

impl CurrencyAmount {
    pub fn from_usd(value: f64) -> Self {
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
struct ApiData {
    campaign: Campaign,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ApiResponse {
    data: Option<ApiData>,

    #[serde(default)]
    errors: Vec<ApiError>,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
struct Location {
    line: usize,
    column: usize,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ApiError {
    message: String,
    #[serde(default)]
    locations: Vec<Location>,
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

pub fn fetch_campaign_json() -> Result<String, Report> {
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

    let json = ureq::post(API_URL)
        .send_json(graph_ql_query)?
        .into_string()?;

    Ok(json)
}

pub fn fetch_campaign() -> Result<Campaign, Report> {
    let json = fetch_campaign_json()?;

    let res: ApiResponse = serde_json::from_str(&json)?;
    if let Some(data) = res.data {
        Ok(data.campaign)
    } else {
        let errors: Vec<String> = res.errors.iter().map(|e| format!("{}", e)).collect();
        let errors = errors.join("\n");

        let report = Report::msg(format!("Campaign Query failed:\n{}", errors));
        Err(report)
    }
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
            data: Some(ApiData {
                campaign: Campaign {
                    name: "Relay FM for St. Jude 2021".to_string(),
                    description: DESCRIPTION.to_string(),
                    slug: "relay-st-jude-21".to_string(),
                    status: "published".to_string(),

                    goal: CurrencyAmount::from_usd(333_333.33),
                    total_amount_raised: CurrencyAmount::from_usd(22_663.40),

                    milestones: vec![
                        Milestone {
                            amount: CurrencyAmount::from_usd(75_000.00),
                            name: "Stephen & Myke go to space via KSP".to_string(),
                        },
                        Milestone {
                            amount: CurrencyAmount::from_usd(55_000.00),
                            name: "Stephen dissembles his NeXTCube on stream".to_string(),
                        },
                        Milestone {
                            amount: CurrencyAmount::from_usd(20_000.00),
                            name: "Myke and Stephen attempt Flight Simulator again".to_string(),
                        },
                        Milestone {
                            amount: CurrencyAmount::from_usd(196_060.44),
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