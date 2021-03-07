#[macro_use]
extern crate serial_test;

mod common;

use anyhow::{anyhow, Result};
use assert_cmd::prelude::*;
use common::{correct_client, correct_server, incorrect_ca_client, incorrect_certificate_client};
use predicates::prelude::*;
use std::panic;

#[test]
#[serial]
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

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}

#[test]
#[serial]
fn run_fails_when_process_doesnt_exist() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = correct_client().unwrap();

        let cmd = client.arg("run").arg("idontexistnoowhere");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Error"));
    });

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}

// mark root-dependent tests as ignored
// can be executed with cargo test --ignored

#[test]
#[serial]
#[ignore]
fn running_under_constrained_memory_works() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = correct_client().unwrap();

        // 7MB = 7340032bytes should be enough for bash to run
        // on any system

        let output = client
            .args(vec![
                "run", "--memory", "7340032", "--", "bash", "-c", "sleep 60",
            ])
            .output()
            .unwrap();

        let id = std::str::from_utf8(&output.stdout).unwrap().trim();

        let mut client = correct_client().unwrap();
        let cmd = client.arg("status").arg(id);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Running"));
    });

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}

#[test]
#[serial]
#[ignore]
fn running_under_constrained_cpu_works() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = correct_client().unwrap();

        let output = client
            .args(vec!["run", "--cpu", "100", "--", "bash", "-c", "sleep 60"])
            .output()
            .unwrap();

        let id = std::str::from_utf8(&output.stdout).unwrap().trim();

        let mut client = correct_client().unwrap();
        let cmd = client.arg("status").arg(id);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Running"));
    });

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}

#[test]
#[serial]
#[ignore]
fn running_under_constrained_memory_makes_cmd_fail_when_going_over() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = correct_client().unwrap();

        // certainly bash needs more than 10 bytes:

        let output = client
            .args(vec![
                "run", "--memory", "10", "--", "bash", "-c", "sleep 60",
            ])
            .output()
            .unwrap();

        let id = std::str::from_utf8(&output.stdout).unwrap().trim();

        let mut client = correct_client().unwrap();
        let cmd = client.arg("status").arg(id);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Killed with signal: 9"));
    });

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}

#[test]
#[serial]
#[ignore]
fn running_under_constrained_disk_works() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = correct_client().unwrap();

        let output = client
            .args(vec!["run", "--disk", "100", "--", "bash", "-c", "sleep 60"])
            .output()
            .unwrap();

        let id = std::str::from_utf8(&output.stdout).unwrap().trim();

        let mut client = correct_client().unwrap();
        let cmd = client.arg("status").arg(id);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Running"));
    });

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}

#[test]
#[serial]
fn status_returns_running_when_running() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = correct_client().unwrap();

        let output = client.args(vec!["run", "sleep", "999"]).output().unwrap();
        let id = std::str::from_utf8(&output.stdout).unwrap().trim();

        let mut client = correct_client().unwrap();
        let cmd = client.arg("status").arg(id);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Running"));
    });

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}

#[test]
#[serial]
fn log_streams_the_logs() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = correct_client().unwrap();

        let output = client.args(vec!["run", "seq", "1", "4"]).output().unwrap();
        let id = std::str::from_utf8(&output.stdout).unwrap().trim();

        let mut client = correct_client().unwrap();
        let cmd = client.arg("log").arg(id).arg("stdout");

        cmd.assert().success().stdout(predicate::str::contains("3"));
    });

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}

#[test]
#[serial]
fn pointing_at_invalid_ca_makes_client_fail() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = incorrect_ca_client().unwrap();

        let cmd = client.arg("run").arg("seq").arg("1").arg("10");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("invalid certificate"));
    });

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}

#[test]
#[serial]
fn pointing_at_invalid_certifiate_makes_client_fail() -> Result<()> {
    let mut server = correct_server()?;
    let mut server_child = server.spawn()?;

    let result = panic::catch_unwind(move || {
        let mut client = incorrect_certificate_client().unwrap();

        let cmd = client.arg("run").arg("seq").arg("1").arg("10");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Unauthorized"));
    });

    server_child.kill().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("panic occurred")),
    }
}
