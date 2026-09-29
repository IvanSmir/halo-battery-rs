use std::io;

use halo_battery::command_error::CommandError;
use serde_json::json;

#[test]
fn displays_the_message_of_each_kind() {
    assert_eq!(CommandError::Unavailable("no update".into()).to_string(), "no update");
    assert_eq!(CommandError::Failed("report not written".into()).to_string(), "report not written");
    assert_eq!(CommandError::Io(io::Error::other("disk full")).to_string(), "disk full");
}

#[test]
fn an_io_error_converts_with_the_question_mark() {
    fn write() -> Result<(), CommandError> {
        Err(io::Error::other("denied"))?;
        Ok(())
    }
    assert!(matches!(write(), Err(CommandError::Io(_))));
}

#[test]
fn serializes_as_kind_and_message() {
    let cases = [
        (CommandError::Unavailable("gone".into()), json!({ "kind": "unavailable", "message": "gone" })),
        (CommandError::Failed("broke".into()), json!({ "kind": "failed", "message": "broke" })),
        (CommandError::Io(io::Error::other("denied")), json!({ "kind": "io", "message": "denied" })),
    ];
    for (error, expected) in cases {
        assert_eq!(serde_json::to_value(&error).unwrap(), expected);
    }
}
