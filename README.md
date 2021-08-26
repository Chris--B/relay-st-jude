# Relay FM St Jude Fundraiser

Every year, Relay FM does a fundraiser to help St Jude and its mission. Learn more about it
[here](https://tiltify.com/@relay-fm/relay-st-jude-21).

This is a Rust crate to fetch the campaign progress and then do stuff with it. It includes a small
library and a sample CLI tool.

Other resources:
- [Building a Donation Tracker Widget with Scriptable](https://zachknox.com/2021/08/21/building-a-donation-tracker-widget)

## Building

This crate is a pretty standard Rust crate and follows the typical build process.

Install Rust with https://rustup.rs/. Reopen your terminal if you've already cloned this repo.

### The CLI tool

Run it from here:
```bash
$ cargo run
```

Install it into your path with cargo
```bash
$ cargo install --path .
```

<img width="650" alt="image" src="https://user-images.githubusercontent.com/1052157/130894036-9b956dc5-a8d5-4345-8325-d055114e13ff.png">


### The library

Add this crate as a dependency by adding this to your `Cargo.toml`

```toml
[dependencies]
relay-st-jude = { git = "https://github.com/Chris--B/relay-st-jude.git" }
```
