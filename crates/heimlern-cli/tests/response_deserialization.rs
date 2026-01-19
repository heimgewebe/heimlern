use heimlern_core::event::AussenEvent;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct ChronikEnvelope {
    payload: AussenEvent,
}

#[derive(Deserialize, Debug)]
struct BatchMeta {
    count: u32,
    generated_at: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ChronikEventsResponse {
    events: Vec<ChronikEnvelope>,
    next_cursor: Option<String>,
    has_more: bool,
    meta: Option<BatchMeta>,
}

#[test]
fn test_chronik_response_deserialization_standard() {
    let json = r#"
    {
        "events": [
            {
                "payload": {
                    "type": "test",
                    "source": "unit_test",
                    "ts": "2023-01-01T12:00:00Z",
                    "id": "event_1"
                }
            }
        ],
        "next_cursor": "token_123",
        "has_more": true
    }
    "#;

    let response: ChronikEventsResponse =
        serde_json::from_str(json).expect("Failed to deserialize response");

    assert_eq!(response.events.len(), 1);
    assert_eq!(response.events[0].payload.r#type, "test");
    assert_eq!(response.next_cursor, Some("token_123".to_string()));
    assert!(response.has_more);
}

#[test]
fn test_chronik_response_deserialization_null_cursor() {
    let json = r#"
    {
        "events": [],
        "next_cursor": null,
        "has_more": false
    }
    "#;

    let response: ChronikEventsResponse =
        serde_json::from_str(json).expect("Failed to deserialize response");

    assert_eq!(response.events.len(), 0);
    assert_eq!(response.next_cursor, None);
    assert!(!response.has_more);
}

#[test]
fn test_chronik_response_deserialization_with_meta() {
    let json = r#"
    {
        "events": [],
        "next_cursor": "token_456",
        "has_more": false,
        "meta": {
            "count": 0,
            "generated_at": "2023-01-01T12:00:00Z"
        }
    }
    "#;

    let response: ChronikEventsResponse =
        serde_json::from_str(json).expect("Failed to deserialize response");

    assert!(response.meta.is_some());
    assert_eq!(response.meta.unwrap().count, 0);
}
