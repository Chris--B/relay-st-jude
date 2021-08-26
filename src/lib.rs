use color_eyre::Report;
use num_format::{Locale, ToFormattedString};
use serde::{de, Deserialize, Deserializer};
use serde_json::json;

use std::fmt;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Milestone {
    #[serde(rename = "name")]
    pub description: String,
    pub amount: Currency,
}

#[derive(Deserialize, Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Currency {
    /// Amount that this represents
    #[serde(deserialize_with = "deserialize_f64_from_str", rename = "value")]
    amount: f64,
}

impl Currency {
    pub fn from_usd(amount: f64) -> Self {
        Self { amount }
    }

    pub fn usd(&self) -> f64 {
        self.amount
    }
}

fn deserialize_f64_from_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(de::Error::custom)
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // I can't figure out how to format with commas and a fixed amount of decimals...
        // So we'll format two ints instead
        let dollars: u64 = self.amount as u64;
        let cents: u8 = (100. * self.amount.fract() + 0.005) as u8;

        // This is our main dollar amount as a string, hurray!
        // We'll use this string and apply width to it directly.
        let s = format!("${}.{:02}", dollars.to_formatted_string(&Locale::en), cents);

        if let Some(width) = f.width() {
            write!(f, "{:>width$}", s, width = width,)
        } else {
            // No width requested? Write it direct
            write!(f, "{}", s)
        }
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

fn build_graph_ql_query(vanity: &str, slug: &str) -> serde_json::Value {
    json!({
        "operationName": "get_campaign_by_vanity_and_slug",
        "variables": {
            "vanity": vanity,
            "slug": slug,
        },
        "query": indoc::indoc!(r#"query get_campaign_by_vanity_and_slug($vanity: String, $slug: String) {
            campaign(vanity: $vanity, slug: $slug) {
                name
                slug
                status
                description
                totalAmountRaised {
                    currency
                    value
                }
                goal {
                    currency
                    value
                }
                milestones {
                    name
                    amount {
                        currency
                        value
                    }
                }
            }
        }"#)
    })
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Campaign {
    pub name: String,

    pub description: String,
    pub slug: String,
    pub status: String,

    pub goal: Currency,
    #[serde(rename = "totalAmountRaised")]
    pub total_amount_raised: Currency,

    #[serde(default)]
    pub milestones: Vec<Milestone>,
}

impl Campaign {
    pub fn fetch() -> Result<Self, Report> {
        Self::fetch_by("@relay-fm", "relay-st-jude-21")
    }

    pub fn fetch_by(vanity: &str, slug: &str) -> Result<Self, Report> {
        // TODO: Don't hard code these, maybe take them from Clap or something.
        let json = Self::fetch_json(vanity, slug)?;

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

    pub fn fetch_json(vanity: &str, slug: &str) -> Result<String, Report> {
        const API_URL: &str = "https://api.tiltify.com";

        let json = ureq::post(API_URL)
            .send_json(build_graph_ql_query(vanity, slug))?
            .into_string()?;

        Ok(json)
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

                    goal: Currency::from_usd(333_333.33),
                    total_amount_raised: Currency::from_usd(22_663.40),

                    milestones: vec![
                        Milestone {
                            amount: Currency::from_usd(75_000.00),
                            description: "Stephen & Myke go to space via KSP".to_string(),
                        },
                        Milestone {
                            amount: Currency::from_usd(55_000.00),
                            description: "Stephen dissembles his NeXTCube on stream".to_string(),
                        },
                        Milestone {
                            amount: Currency::from_usd(20_000.00),
                            description: "Myke and Stephen attempt Flight Simulator again"
                                .to_string(),
                        },
                        Milestone {
                            amount: Currency::from_usd(196_060.44),
                            description: "$1 million raised in 3 years!".to_string(),
                        },
                    ],
                },
            }),

            errors: vec![],
        };

        assert_eq!(expected, serde_json::from_str(RESPONSE).unwrap());
    }

    /// Verify that the live API JSON from the API matches our serde model
    #[test]
    fn live_response() {
        let _campaign = Campaign::fetch().unwrap();
    }
}