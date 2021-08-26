//! Types and methods to query Relay St Jude campaign status
//!
//! This crate lets you query the status of the Relay St Jude campaign and programmatically
//! display the progress and milestones of the fund-raising effort.
//!
//! To learn more about St Jude, see the
//! [2021 Campaign Page](https://tiltify.com/@relay-fm/relay-st-jude-21).
//!
//! # Usage
//! The main type to use is [`Campaign`](crate::Campaign):
//! ```rust
//! use relay_st_jude::Campaign;
//!
//! // Fetch live campaign data. This requires a network connection.
//! let campaign: Campaign = Campaign::fetch().unwrap();
//!
//! // Currency amounts come in USD
//! let current: f64 = campaign.total_amount_raised.usd();
//! let goal: f64 = campaign.goal.usd();
//!
//! // Do something interesting with the data!
//! println!("${:.2}% raised so far!", 100. * current / goal );
//! ```

#![warn(missing_docs)]

use color_eyre::Report;
use num_format::{Locale, ToFormattedString};
use serde::{de, Deserialize, Deserializer};
use serde_json::json;

use std::fmt;

/// A fund-raising milestone
///
/// New events are unlocked when this milestone is reached. Check
/// [`Milestone::description`](crate::Milestone::description) for details on the event.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Milestone {
    /// A description of an event that the hosts did or will do when the milestone is reached
    #[serde(rename = "name")]
    pub description: String,

    /// The amount, in USD, for this milestone
    pub amount: Usd,
}

/// A dollar amount expressed in United States Dollar (USD)
#[derive(Deserialize, Copy, Clone, Debug, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Usd {
    // TODO: We'd like to make this a tuple-struct, but need to sort out the serde logic first
    #[serde(deserialize_with = "deserialize_f64_from_str", rename = "value")]
    amount: f64,
}

impl Usd {
    /// Construct from a dollar amount
    pub fn new(amount: f64) -> Self {
        Self { amount }
    }

    /// Convert into a number for arithmetic
    ///
    /// This amount is in USD (surprise!)
    pub fn usd(&self) -> f64 {
        self.amount
    }
}

/// The GraphQL API has a complex, fully generic Currency type.
/// I don't care, I just want USD. 🎇🇺🇸🦅🎆
///
/// I'm sure I'll regret this later.
fn deserialize_f64_from_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(de::Error::custom)
}

/// Display the Usd amount in a typical currency fashion
impl fmt::Display for Usd {
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

// TODO: This should probably be an enum - both fields here are mutually exclusive
#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ApiResponse {
    data: Option<ApiData>,

    #[serde(default)]
    errors: Vec<ApiError>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ApiData {
    campaign: Campaign,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ApiError {
    message: String,
    #[serde(default)]
    locations: Vec<Location>,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
struct Location {
    line: usize,
    column: usize,
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

/// A fund raising campaign for a good cause
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Campaign {
    /// Registered name of the campaign
    ///
    /// Useful for uniquely identifying a campaign for humans
    pub name: String,

    /// A description of what this campaign is for and about
    pub description: String,

    /// The current amount of money raised
    #[serde(rename = "totalAmountRaised")]
    pub total_amount_raised: Usd,

    /// The goal for money raised
    pub goal: Usd,

    /// A list of milestones set for the campaign currently, including their progress
    #[serde(default)]
    pub milestones: Vec<Milestone>,
}

impl Campaign {
    /// Fetch the Relay St Jude campaign status from online
    pub fn fetch() -> Result<Self, Report> {
        Self::fetch_by("@relay-fm", "relay-st-jude-21")
    }

    /// Fetch an arbitrary vanity & slug from online
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

    /// Fetch just the json for a vanity & slug from online
    ///
    /// This can be parsed into a [`Campaign`](Campaign) object, but prefer calling
    /// [`fetch`](Campaign::fetch) directly or [`fetch_by`](Campaign::fetch_by).
    ///
    /// Use this if you're getting deserialization errors.
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

                    goal: Usd::new(333_333.33),
                    total_amount_raised: Usd::new(22_663.40),

                    milestones: vec![
                        Milestone {
                            amount: Usd::new(75_000.00),
                            description: "Stephen & Myke go to space via KSP".to_string(),
                        },
                        Milestone {
                            amount: Usd::new(55_000.00),
                            description: "Stephen dissembles his NeXTCube on stream".to_string(),
                        },
                        Milestone {
                            amount: Usd::new(20_000.00),
                            description: "Myke and Stephen attempt Flight Simulator again"
                                .to_string(),
                        },
                        Milestone {
                            amount: Usd::new(196_060.44),
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
        let campaign = Campaign::fetch().unwrap();
        dbg!(campaign);
    }
}
