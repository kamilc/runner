mod cli;
mod runner;

use anyhow::{anyhow, Context, Result};
use cli::client::{Cli, Command, Descriptor};
use std::io::Write;
use structopt::StructOpt;
use tonic::transport::Uri;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};

use crate::runner::service::{
    log_request, log_response, run_request, run_response, runner_client, status_response,
    LogRequest, RunRequest, StatusRequest, StopRequest,
};

fn main() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { run().await })
        .map_err(|err| {
            eprintln!("{}", err.to_string());
            std::process::exit(1);
        })
}

async fn run() -> Result<()> {
    let args = Cli::from_args();

    let cert = tokio::fs::read(args.cert).await?;
    let key = tokio::fs::read(args.key).await?;

    let client_identity = Identity::from_pem(cert, key);

    let server_ca_cert = tokio::fs::read(args.server_ca).await?;
    let server_ca_cert = Certificate::from_pem(server_ca_cert);

    let tls = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(server_ca_cert)
        .identity(client_identity);

    let uri = args
        .address
        .parse::<Uri>()
        .context("Invalid address given")?;

    let channel = Channel::builder(uri).tls_config(tls)?.connect().await?;

    let mut client = runner_client::RunnerClient::new(channel);

    match args.command {
        Command::Run {
            memory,
            disk,
            cpu,
            command,
            args,
        } => {
            let request = tonic::Request::new(RunRequest {
                command,
                arguments: args,
                disk: disk.map(run_request::Disk::MaxDisk),
                memory: memory.map(run_request::Memory::MaxMemory),
                cpu: cpu.map(run_request::Cpu::MaxCpu),
            });

            let response = client.run(request).await?;

            match response.into_inner().results.unwrap() {
                run_response::Results::Id(id) => {
                    println!("{}", id);
                    Ok(())
                }
                run_response::Results::Error(err) => {
                    Err(anyhow!("Error: {}", err.description))
                    //eprintln!("Error: {}", err.description)
                }
            }
        }
        Command::Stop { id } => {
            let request = tonic::Request::new(StopRequest { id: id.to_string() });

            let response = client.stop(request).await?;

            match response.into_inner().error {
                Some(err) => {
                    //println!("Error: {}", err.description)
                    Err(anyhow!("Error: {}", err.description))
                }
                None => {
                    println!("Stopped");
                    Ok(())
                }
            }
        }
        Command::Status { id } => {
            let request = tonic::Request::new(StatusRequest { id: id.to_string() });

            let response = client.status(request).await?;

            match response.into_inner().results.unwrap() {
                status_response::Results::Result(result) => match result.finish {
                    Some(status_response::status_result::Finish::Result(exit_result)) => {
                        if let Some(status_response::status_result::exit_result::Exit::Code(code)) =
                            exit_result.exit
                        {
                            println!("Exited with code: {}", code);
                        } else if let Some(
                            status_response::status_result::exit_result::Kill::Signal(signal),
                        ) = exit_result.kill
                        {
                            println!("Killed with signal: {}", signal);
                        } else {
                            println!("Stopped but no exit code or signal is known");
                        }
                        Ok(())
                    }
                    None => {
                        println!("Running");
                        Ok(())
                    }
                },
                status_response::Results::Error(err) => {
                    //println!("Error: {}", err.description);
                    Err(anyhow!("Error: {}", err.description))
                }
            }
        }
        Command::Log { id, descriptor } => {
            let descriptor = match descriptor {
                Descriptor::Stdout => log_request::Descriptor::Stdout as i32,
                Descriptor::Stderr => log_request::Descriptor::Stderr as i32,
            };
            let request = tonic::Request::new(LogRequest {
                id: id.to_string(),
                descriptor,
            });

            let response = client.log(request).await?;
            let mut inbound = response.into_inner();
            let mut out = std::io::stdout();

            while let Some(item) = inbound.message().await? {
                match item.results.unwrap() {
                    log_response::Results::Data(data) => {
                        out.write_all(&data)
                            .context("Unable to write data into the stdout")?;
                    }
                    log_response::Results::Error(err) => {
                        return Err(anyhow!("Error: {}", err.description));
                    }
                }
            }

            Ok(())
        }
    }
}
