use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CalendarResponse {
    data: Vec<CalendarEvent>,
}

#[derive(Debug, Deserialize)]
struct CalendarEvent {
    name: String,
    date: Option<String>,
    announcement_datetime_utc: Option<String>,
    market_tier: Option<u8>,
    top_tier_for_currency: Option<bool>,
}

impl CalendarEvent {
    fn is_top_tier(&self) -> bool {
        self.top_tier_for_currency.unwrap_or(false) || self.market_tier == Some(1)
    }

    fn event_date(&self) -> Option<&str> {
        self.announcement_datetime_utc
            .as_deref()
            .or(self.date.as_deref())
            .map(|value| &value[..10.min(value.len())])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://fxmacrodata.com/api/v1/calendar/USD?start_date=2026-07-01&end_date=2026-07-20";
    let calendar = reqwest::get(url)
        .await?
        .error_for_status()?
        .json::<CalendarResponse>()
        .await?;

    println!("Top-tier USD macro blackout dates:");
    for event in calendar.data.iter().filter(|event| event.is_top_tier()) {
        if let Some(event_date) = event.event_date() {
            println!("  {event_date}: {}", event.name);
        }
    }

    Ok(())
}
