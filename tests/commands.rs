mod common;

use anyhow::{anyhow, Context, Result};
use assert_cmd::prelude::*;
use common::{correct_client, correct_server};
use predicates::prelude::*;
use std::panic;
use std::process::Command;

#[test]
fn run_returns_a_uuid_when_correct() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = correct_client().unwrap();

        let cmd = client.arg("run").arg("seq").arg("1").arg("10");

        cmd.assert().success().stdout(
            predicate::str::is_match(
                "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
            )
            .unwrap(),
        );
    });

    server_child.kill();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}
