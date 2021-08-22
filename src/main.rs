use color_eyre::Report;

mod campaign;
use campaign::{fetch_campaign, Campaign};

fn percent(a: campaign::Currency, b: campaign::Currency) -> String {
    format!("{:2.1}%", 100.0 * a.usd() / b.usd())
}

fn dollars(a: campaign::Currency) -> String {
    format!("${:.2}", a.usd())
}

fn main() -> Result<(), Report> {
    setup()?;

    let mut campaign: Campaign = fetch_campaign()?;

    // Sort milestones by $$
    campaign
        .milestones
        .sort_by_key(|milestone| (milestone.amount.usd() * 100.) as u64);

    println!("{}!", campaign.name);
    println!(
        "{} of {}",
        dollars(campaign.total_amount_raised),
        dollars(campaign.goal)
    );

    for milestone in &campaign.milestones {
        print!("    ");

        if milestone.amount < campaign.total_amount_raised {
            print!("  ✅ ");
        } else {
            print!(
                "{}",
                percent(campaign.total_amount_raised, milestone.amount)
            );
        }
        print!(" ");

        println!("{:>10}: {}", dollars(milestone.amount), milestone.name);
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
