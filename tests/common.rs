use anyhow::Result;
use assert_cmd::prelude::*;
use std::process::Command;

pub fn correct_server() -> Result<Command> {
    let mut server = Command::cargo_bin("server")?;

    server
        .arg("--address")
        .arg("[::1]:50052")
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
        .arg("--address")
        .arg("dns://[::1]:50052")
        .arg("--cert")
        .arg("example/client.pem")
        .arg("--server-ca")
        .arg("example/ca.pem")
        .arg("--key")
        .arg("example/client.p8");

    Ok(client)
}

pub fn incorrect_certificate_client() -> Result<Command> {
    let mut client = Command::cargo_bin("client")?;

    client
        .arg("--address")
        .arg("dns://[::1]:50052")
        .arg("--cert")
        .arg("example/client.pem")
        .arg("--server-ca")
        .arg("example/ca.other.pem")
        .arg("--key")
        .arg("example/client.p8");

    Ok(client)
}
