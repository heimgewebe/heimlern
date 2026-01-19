use heimlern_core::event::AussenEvent;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct ChronikEnvelope {
    payload: AussenEvent,
}

#[derive(Deserialize, Debug)]
struct ChronikEventsResponse {
    events: Vec<ChronikEnvelope>,
    next_cursor: u64,
    has_more: bool,
}

#[test]
fn test_chronik_response_deserialization() {
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
            },
            {
                "payload": {
                    "type": "test_2",
                    "source": "unit_test",
                    "id": "event_2"
                }
            }
        ],
        "next_cursor": 12345,
        "has_more": true
    }
    "#;

    let response: ChronikEventsResponse =
        serde_json::from_str(json).expect("Failed to deserialize response");

    assert_eq!(response.events.len(), 2);
    assert_eq!(response.events[0].payload.r#type, "test");
    assert_eq!(response.next_cursor, 12345);
    assert!(response.has_more);
}
