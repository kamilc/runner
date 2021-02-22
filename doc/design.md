# Runner - a job worker service

This document describes a job worker service and its client. It allows its users to run arbitrary Linux processes under specified resource constraints.

## Scope

The final solution consists of a server and a client. The client connects to the server and allows the following set of requests:

- start a process
- stop it
- query its status
- get the stream of its output

The last two points apply also to processes that were already finished. The stdout and stderr are gathered separately. The streaming into the client is one-file-descriptor-at-a-time. The streaming command allows a user to choose stdout vs stderr with stdout being default.

The communication between the server and client utilizes gRPC as its request-response protocol. It employs mutual TLS for authentication. It also performs simple authorization checks, letting the server accept or reject requests.

The scheduled processes are constrained in the amount of CPU, memory and disk IO they should be allowed. The amounts for constraints are requested by the client at the time of the job being created. The disk IO constraint is applied to both reads and writes.

The final solution only supports Linux and is CLI based. As it's a proof of concept work, otpimizations such as request throttling or caching are considered out of scope. The logs for processes are stored on disk in plain-text without any encryption. The processes always run under the UID of the server. The UID of the server is assumed to be 0 always due to requirements around the resource constraining. It does a simple check for the UID in order to provide user with friendly error messages. Keeping the list of processes between server restarts is outside of the scope. Scheduled processes are executed immediately, with no ability to specify a point-in-time.

## Technical details

All processes known to the server have a UUID identifying them. This value is initially returned to the client upon the job creation. All other requests require this value as part of the request payload in order to identify the process in question.

The list of possible request results:

- TLS authentication error
- authorization error
- given process id is not a valid UUID
- (any) task-specific error
- OS unexpected error (when e.g. the OS refuses some operation that the server needs to perform, e.g. writing to log file because the device is full)
- success

Task specific arguments and errors

- Task: Start a process
  - Arguments:
    - Command name
    - Command arguments (a list)
    - Max memory
    - Max CPU
    - Disk major
    - Disk minor
    - Max disk IO
  - Errors:
    - Given command name is empty
    - One of given command arguments is empty
    - Invalid max memory value
    - Invalid max CPU value
    - Invalid disk minor
    - Invalid disk major
    - Invalid max disk IO value
    - Couldn't start a process
  - Returns:
    - A UUID value of the scheduled job
- Task: Stop a process
  - Arguments:
    - A UUID of the process
  - Errors:
    - Process not found
    - Process already stopped
    - Couldn't stop a process
  - Returns:
    - Aknowledgement of the process being stopped
- Task: Query process status
  - Arguments:
    - A UUID of the process
  - Errors:
    - Process not found
  - Returns:
    - One of the two values: Running, Stopped
- Task: Show the process output
  - Arguments:
    - A UUID of the process
    - A file descriptor to use: Stdout, Stderr
  - Errors:
    - Process not found
  - Returns:
    - A stream of the process output at a given file descriptor. The stream is "followed" until Ctrl-C is used in the client.

The authorization step is very basic and is based on the specific value in the client's certificate subject.

## Implementation details

The solution will be coded in Rust, using latest versions of gRPC and TLS libraries: tonic and rustls. Simple and robust command arguments handling will be provided by the structopt crate. Additional dependencies will be chosen at a later point.

Brief overview of the technical approach for the tasks to implement follows.

### Task: Start a process

When the process is started, the server adds it (in a thread-safe way) to its internal hash map of processes. The map is keyed with id and valued with the pid. Additionally, two files are created on disk: for storing the stdout and stderr. Each scheduled process has stdout and stderr pointed at these two files. Upon the process creation, a new control group is created and configured as per the constraint parameters. The new process is added to the group before the server responds with the UUID.

### Task: Stop a process

The server tries to kill the process. When successful, it returns an acknowledgement - otherwise an error message.

### Task: Query process status

The server reads the process pid from the internal map. It then uses it to query the OS for the process status. When it's found, it returns the payload that means "process running", and "process stopped" otherwise.

### Task: Show the process output

The server reads from the relevant output file, streaming the contents via gRPC. When no new data is encountered it sleeps for a couple hundreds milliseconds and tried to poll for new data. It all happens in a loop and only stops upon the Ctrl-C from the user.
