use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Deserialize;
use std::time::Duration as RequestTimeout;

#[derive(Debug, Deserialize)]
#[serde(try_from = "CalendarResponseBody")]
struct CalendarResponse {
    data: Vec<CalendarEvent>,
}

#[derive(Deserialize)]
struct CalendarResponseBody {
    data: Vec<CalendarEvent>,
    error: Option<serde_json::Value>,
    detail: Option<serde_json::Value>,
}

impl TryFrom<CalendarResponseBody> for CalendarResponse {
    type Error = &'static str;

    fn try_from(body: CalendarResponseBody) -> Result<Self, Self::Error> {
        if body.error.is_some() || body.detail.is_some() {
            return Err("Calendar response contains an error");
        }
        Ok(Self { data: body.data })
    }
}

async fn parse_calendar(
    response: reqwest::Response,
) -> Result<CalendarResponse, Box<dyn std::error::Error>> {
    if !response.status().is_success() {
        return Err(format!("Calendar request failed with HTTP {}", response.status().as_u16()).into());
    }
    Ok(response.json::<CalendarResponse>().await?)
}

#[derive(Debug, Deserialize)]
struct CalendarEvent {
    release: String,
    name: Option<String>,
    announcement_datetime: Option<i64>,
    market_tier: Option<u8>,
    top_tier_for_currency: Option<bool>,
}

impl CalendarEvent {
    fn is_top_tier(&self) -> bool {
        self.top_tier_for_currency.unwrap_or(false) || self.market_tier == Some(1)
    }

    fn event_date(&self) -> Option<NaiveDate> {
        DateTime::<Utc>::from_timestamp(self.announcement_datetime?, 0)
            .map(|timestamp| timestamp.date_naive())
    }

    fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&self.release)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (start, end) = match args.as_slice() {
        [] => {
            let start = Utc::now().date_naive();
            (start, start + Duration::days(14))
        }
        [start, end] => (start.parse::<NaiveDate>()?, end.parse::<NaiveDate>()?),
        _ => return Err("Usage: fxmacrodata_release_calendar [YYYY-MM-DD YYYY-MM-DD]".into()),
    };
    if end < start {
        return Err("End date must not precede start date".into());
    }
    let client = reqwest::Client::builder()
        .timeout(RequestTimeout::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let response = client
        .get("https://api.fxmacrodata.com/v1/calendar/usd")
        .query(&[("start_date", start.to_string()), ("end_date", end.to_string())])
        .send()
        .await?;
    let calendar = parse_calendar(response).await?;

    println!("Top-tier USD macro blackout dates (UTC):");
    for event in calendar.data.iter().filter(|event| event.is_top_tier()) {
        if let Some(event_date) = event.event_date() {
            println!("  {event_date}: {}", event.display_name());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_name_and_formatted_timestamp_are_not_required() {
        let response: CalendarResponse = serde_json::from_str(
            r#"{"data":[{"release":"synthetic_release","announcement_datetime":0,"market_tier":1}]}"#,
        )
        .unwrap();
        let event = &response.data[0];
        assert_eq!(event.display_name(), "synthetic_release");
        assert_eq!(event.event_date(), NaiveDate::from_ymd_opt(1970, 1, 1));
        assert!(event.is_top_tier());
    }

    #[test]
    fn null_name_falls_back_to_release_and_reference_period_is_not_event_time() {
        let event: CalendarEvent = serde_json::from_str(
            r#"{"release":"synthetic_release","name":null,"date":"1969-12-01","announcement_datetime":0}"#,
        )
        .unwrap();
        assert_eq!(event.display_name(), "synthetic_release");
        assert_eq!(event.event_date(), NaiveDate::from_ymd_opt(1970, 1, 1));
    }

    #[test]
    fn unknown_or_out_of_range_event_time_is_not_guessed() {
        for payload in [
            r#"{"release":"synthetic_release","date":"1970-01-01"}"#,
            r#"{"release":"synthetic_release","announcement_datetime":9223372036854775807}"#,
        ] {
            let event: CalendarEvent = serde_json::from_str(payload).unwrap();
            assert_eq!(event.event_date(), None);
        }
    }

    #[test]
    fn unicode_name_is_preserved_without_slicing_utf8() {
        let event: CalendarEvent = serde_json::from_str(
            r#"{"release":"synthetic_release","name":"日本語","announcement_datetime":0,"top_tier_for_currency":true}"#,
        )
        .unwrap();
        assert_eq!(event.display_name(), "日本語");
        assert!(event.is_top_tier());
    }
}

#[cfg(test)]
mod envelope_tests {
    use super::CalendarResponse;

    fn rejects(payload: &str) {
        let error = serde_json::from_str::<CalendarResponse>(payload).unwrap_err();
        assert!(error.to_string().contains("Calendar response contains an error"));
        assert!(!error.to_string().contains("private-provider-marker"));
    }

    #[test]
    fn rejects_string_error_with_empty_data() {
        rejects(r#"{"data":[],"error":"private-provider-marker"}"#);
    }

    #[test]
    fn rejects_object_error_with_empty_data() {
        rejects(r#"{"data":[],"error":{"message":"private-provider-marker"}}"#);
    }

    #[test]
    fn rejects_detail_string_with_empty_data() {
        rejects(r#"{"data":[],"detail":"private-provider-marker"}"#);
    }

    #[test]
    fn rejects_detail_object_with_empty_data() {
        rejects(r#"{"data":[],"detail":{"message":"private-provider-marker"}}"#);
    }

    #[test]
    fn rejects_error_even_with_calendar_rows() {
        rejects(r#"{"data":[{"release":"synthetic","announcement_datetime":0}],"error":"private-provider-marker"}"#);
    }

    #[test]
    fn preserves_genuine_empty_success() {
        assert!(serde_json::from_str::<CalendarResponse>(r#"{"data":[]}"#).unwrap().data.is_empty());
    }

    #[test]
    fn preserves_null_error_metadata() {
        assert!(serde_json::from_str::<CalendarResponse>(r#"{"data":[],"error":null,"detail":null}"#).unwrap().data.is_empty());
    }
}

#[cfg(test)]
mod response_status_tests {
    use super::parse_calendar;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    async fn response(status: u16) -> reqwest::Response {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            assert!(stream.read(&mut request).unwrap() > 0);
            let body = r#"{"data":[]}"#;
            let output = format!("HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}", body.len());
            stream.write_all(output.as_bytes()).unwrap();
        });
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(5))
            .build().unwrap()
            .get(format!("http://{address}/calendar"))
            .send().await.unwrap()
    }

    #[tokio::test]
    async fn rejects_redirect_even_when_body_is_valid_calendar_json() {
        let error = parse_calendar(response(303).await).await.unwrap_err();
        assert_eq!(error.to_string(), "Calendar request failed with HTTP 303");
    }

    #[tokio::test]
    async fn accepts_successful_http_calendar() {
        assert!(parse_calendar(response(200).await).await.unwrap().data.is_empty());
    }
}
