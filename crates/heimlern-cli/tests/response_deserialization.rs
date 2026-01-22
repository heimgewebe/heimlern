use heimlern_core::event::AussenEvent;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct ChronikEnvelope {
    r#type: Option<String>,
    payload: AussenEvent,
}

#[derive(Deserialize, Debug)]
struct BatchMeta {
    count: Option<u32>,
    #[allow(dead_code)]
    generated_at: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ChronikEventsResponse {
    events: Vec<ChronikEnvelope>,
    next_cursor: Option<u64>,
    has_more: bool,
    meta: Option<BatchMeta>,
}

#[test]
fn test_chronik_response_deserialization_standard() {
    let json = r#"
    {
        "events": [
            {
                "type": "test_type_wrapper",
                "payload": {
                    "type": "test",
                    "source": "unit_test",
                    "ts": "2023-01-01T12:00:00Z",
                    "id": "event_1"
                }
            }
        ],
        "next_cursor": 123,
        "has_more": true
    }
    "#;

    let response: ChronikEventsResponse = serde_json::from_str(json).expect("Failed to deserialize response");

    assert_eq!(response.events.len(), 1);
    assert_eq!(response.events[0].r#type, Some("test_type_wrapper".to_string()));
    assert_eq!(response.events[0].payload.r#type, "test");
    assert_eq!(response.next_cursor, Some(123));
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

    let response: ChronikEventsResponse = serde_json::from_str(json).expect("Failed to deserialize response");

    assert_eq!(response.events.len(), 0);
    assert_eq!(response.next_cursor, None);
    assert!(!response.has_more);
}

#[test]
fn test_chronik_response_deserialization_with_meta() {
    let json = r#"
    {
        "events": [],
        "next_cursor": 456,
        "has_more": false,
        "meta": {
            "count": 0,
            "generated_at": "2023-01-01T12:00:00Z"
        }
    }
    "#;

    let response: ChronikEventsResponse = serde_json::from_str(json).expect("Failed to deserialize response");

    assert!(response.meta.is_some());
    assert_eq!(response.meta.unwrap().count, Some(0));
    assert_eq!(response.next_cursor, Some(456));
}
