use crossterm::event::{KeyCode, read};
use tracing::info;

pub fn enter_to_proceed() {
    loop {
        if let crossterm::event::Event::Key(event) = read().unwrap() {
            match event.code {
                KeyCode::Enter => {
                    break;
                }
                _ => {
                    info!("Cya!");
                    std::process::exit(0);
                }
            }
        }
    }
}

pub fn parse_insufficient_funds_message(msg: &str)-> Option<String> {
    if msg.contains("insufficient funds for gas") {
        let address_start = msg.find("address ").unwrap_or(0) + "address ".len();
        let have_start = msg.find("have ").unwrap_or(0) + "have ".len();
        let want_start = msg.find("want ").unwrap_or(0) + "want ".len();
        let address_end = msg[address_start..].find(' ').unwrap_or(0) + address_start;
        let have_end = msg[have_start..].find(' ').unwrap_or(0) + have_start;
        let want_end = msg[want_start..].find(';').unwrap_or(0) + want_start;

        let address = msg[address_start..address_end].trim();
        let have_str = msg[have_start..have_end].trim();
        let want_str = msg[want_start..want_end].trim();

        let have_value = have_str.parse::<f64>().unwrap_or(0.0);
        let want_value = want_str.parse::<f64>().unwrap_or(0.0);

        let required_additional_eth = (want_value - have_value) / 1e18; // converting wei to ETH
        return Some(format!("Insufficient Funds Error: Wallet {} needs an additional {:.18} ETH to cover the transaction costs.", address, required_additional_eth));

    }
    None
}