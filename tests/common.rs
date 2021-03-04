use anyhow::{anyhow, Context, Result};
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

pub fn correct_server() -> Result<Command> {
    let mut server = Command::cargo_bin("server")?;

    server
        .arg("--cert")
        .arg("example/server.pem")
        .arg("--client-ca")
        .arg("example/ca.pem")
        .arg("--key")
        .arg("example/server.p8");

    Ok(server)
}

pub fn correct_client() -> Result<Command> {
    let mut client = Command::cargo_bin("client")?;

    client
        .arg("--cert")
        .arg("example/client.pem")
        .arg("--server-ca")
        .arg("example/ca.pem")
        .arg("--key")
        .arg("example/client.p8");

    Ok(client)
}
