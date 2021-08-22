use color_eyre::Report;

mod campaign;
use campaign::{fetch_campaign, Campaign};

fn main() -> Result<(), Report> {
    setup()?;

    let mut campaign: Campaign = fetch_campaign()?;

    // Sort them by $$
    campaign
        .milestones
        .sort_by_key(|milestone| (milestone.amount.value * 100.) as u64);

    println!("{}!", campaign.name);
    println!(
        "$ {:.2} / {:.2}",
        campaign.total_amount_raised.value, campaign.goal.value
    );

    for milestone in &campaign.milestones {
        print!("    ");

        if milestone.amount.value < campaign.total_amount_raised.value {
            print!("  ✅ ");
        } else {
            print!(
                "{:2.1}%",
                100.0 * campaign.total_amount_raised.value / milestone.amount.value
            );
        }
        print!(" ");

        let dollars = format!("${:.2}", milestone.amount.value);
        println!("{:>10}: {}", dollars, milestone.name);
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
