use color_eyre::Report;

use relay_st_jude::{Campaign, Usd};

fn percent(a: Usd, b: Usd) -> String {
    format!("{:2.1}%", 100.0 * a.usd() / b.usd())
}

fn main() -> Result<(), Report> {
    setup()?;

    let mut campaign: Campaign = Campaign::fetch()?;

    // Sort milestones by $$
    campaign
        .milestones
        .sort_by_key(|milestone| (milestone.amount.usd() * 100.) as u64);

    println!("{}!", campaign.name);
    println!("{} of {}", campaign.total_amount_raised, campaign.goal);

    for milestone in &campaign.milestones {
        // Indent
        print!("    ");

        if milestone.amount < campaign.total_amount_raised {
            // Print an indicator if we've made it
            print!("✅ ");
            // don't print % after we pass it
            print!("     ");
        } else {
            // ... or an indicator if we have not
            print!("🤞 ");
            // print percent for not-yet-completed milestones
            print!(
                "{}",
                percent(campaign.total_amount_raised, milestone.amount)
            );
        }
        // Padding after the above
        print!(" ");

        println!("{:15} - {}", milestone.amount, milestone.description);
    }

    Ok(())
}

fn setup() -> Result<(), Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install()?;

    Ok(())
}
