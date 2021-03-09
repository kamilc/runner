# Runner

A job worker service and its client. It allows its users to run arbitrary Linux processes under specified resource constraints.

## Requirements

* A x86-64 Linux machine with v1 control groups enabled (other architectures were not tested)
* libudev-devel
* pkg-config
* gcc
* make
* openssl (only if you want to use the Makefile to generate example certificates)
* rustc 1.52.0-nightly
* rustfmt (needed to compile protobufs)

### Notes on v1 cgroups

As RHEL8, Centos8 and latest Fedora focus on promoting podman, cgroups v1 are disabled by default (podman uses v2 while its direct competitor, Docker is still on v1). If you're using a system with v1 disabled, you'll need to run the following command to re-enable them:

```bash
grubby --update-kernel=ALL --args="systemd.unified_cgroup_hierarchy=0"
```

You'll then need to restart your system for the changes to get applied.

### Where to find the dependiencies

For Ubuntu related systems:

```bash
sudo apt-get install build-essential libudev-dev
```

For latest Fedora (likely Centos8 and RHEL8 too but I haven't tested):

```bash
sudo dnf install systemd-devel gcc make openssl
```

#### Rustup

If you don't have it installed yet:

```bash
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o ~/rustup.sh
$ sh ~/rustup.sh -y --default-toolchain="nightly"
```

The testing suite that checks features requiring root privileges uses sudo. If you're using `rustup` make sure the toolchain is configured corectly with:

```bash
sudo -E env "PATH=$PATH" rustup default nightly
```

## Compiling

To just compile the code in the debug mode run the following:

```bash
$ make build
```

## Testing

Some tests rely on the certificates being present under the `example` directory. For this reason, it's advisable to run the tests the following way:

```bash
$ make test
```

This generates the certificates if they are not present and runs `cargo test`.

### Testing root requiring features

As resource constraining requires root privileges, the integration tests covering it are marked as ignored
when running the suite by default. There's a special `make` rule helper for running those tests in the
context of a privileged user:

```bash
$ make constraint-test
```

The above command depends on `sudo` being present on the system and the user being in the "sudoers".

## Using the server and the client

First make sure that the binaries have been compiled by running `make build`.

In order to start the server, run the following in one of your terminals:

```bash
$ sudo target/debug/server --cert example/server.pem --client-ca example/ca.pem --key example/server.p8
```

Notice the need for `sudo` as the server needs privileges for creating Linux control groups.

You can also run the server with log messages enabled:

```bash
$ RUST_LOG=info sudo -E target/debug/server --cert example/server.pem --client-ca example/ca.pem --key example/server.p8
```

Now in a separate terminal, use the client as shown below:

Creating a task:

```bash
$ target/debug/client --cert example/client.pem --server-ca example/ca.pem --key example/client.p8 run -- bash -c 'for i in $(seq 1 99); do echo $i; sleep 1; done'
34ea3c1a-3413-4300-9ced-feab108cb5dc
```

Querying its status:

```bash
$ target/debug/client --cert example/client.pem --server-ca example/ca.pem --key example/client.p8 status 34ea3c1a-3413-4300-9ced-feab108cb5dc
Running
```

Examining its logs:

```bash
$ target/debug/client --cert example/client.pem --server-ca example/ca.pem --key example/client.p8 log 34ea3c1a-3413-4300-9ced-feab108cb5dc stdout
1
2
3
4
^C
```

Stopping it:

```bash
$ target/debug/client --cert example/client.pem --server-ca example/ca.pem --key example/client.p8 stop 34ea3c1a-3413-4300-9ced-feab108cb5dc
Stopped
```

## Getting help

At any point, you can list all possible arguments that server and client take with:

```bash
$ target/debug/client --help
runner 0.1.0

USAGE:
    client [OPTIONS] --cert <cert> --key <key> --server-ca <server-ca> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --address <address>        gRPC address [env: SERVER_ADDRESS=]  [default: dns://[::1]:50051]
        --cert <cert>              Path to the client certificate [env: CLIENT_CERT=]
        --cipher <cipher>          Ciphersuite variant: chacha20 or aes [env: CIPHER=]  [default: chacha20]
        --key <key>                Path to the client key [env: CLIENT_KEY=]
        --server-ca <server-ca>    Path to the server's CA root certificate [env: SERVER_CA=]

SUBCOMMANDS:
    help      Prints this message or the help of the given subcommand(s)
    log       Follow command's logs
    run       Run a command
    status    Get command's status
    stop      Stop a command
```

And for the server:

```bash
$ target/debug/server --help
runner 0.1.0

USAGE:
    server [OPTIONS] --cert <cert> --client-ca <client-ca> --key <key>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --address <address>        gRPC address [env: SERVER_ADDRESS=]  [default: [::1]:50051]
        --cert <cert>              Path to the server certificate [env: SERVER_CERT=]
        --cipher <cipher>          Ciphersuite variant: chacha20 or aes [env: CIPHER=]  [default: chacha20]
        --client-ca <client-ca>    Path to the client's CA root certificate [env: CLIENT_CA=]
        --key <key>                Path to the server key [env: SERVER_KEY=]
```
