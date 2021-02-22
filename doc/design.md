# Runner - a job worker service

This document describes a job worker service and its client. It allows its users to run arbitrary Linux processes under specified resource constraints.

## Scope

The final solution consists of a server and a client. The client connects to the server and allows the following set of requests:

- start a process
- stop it
- query its status
- get the stream of its output

The last two points apply also to processes that were already finished. The stdout and stderr are gathered separately. The streaming into the client is one-file-descriptor-at-a-time. The production system could implement interweaving of the two streams as they happen naturally in the terminal but this stays out of the scope of this proof-of-concept work. The streaming command allows a user to choose stdout vs stderr with stdout being defaulted.

The communication between the server and client utilizes gRPC as its request-response protocol. It employs mutual TLS for authentication. It also performs simple authorization checks, letting the server accept or reject requests.

The scheduled processes are constrained in the amount of CPU, memory, and disk IO they should be allowed. This is achieved using the mechanism of Linux control groups. The amounts for constraints are requested by the client at the time of the job being created. The disk IO constraint is applied to both reads and writes. A fuller solution would be to allow a user to specify the constraints for writes and reads separately.

The final solution only supports Linux as it relies heavily on control groups. It is also CLI only.

As it's a proof of concept work, optimizations such as request throttling or caching are considered out of scope. The logs for processes are stored on disk in plain-text without any encryption. No attempt of defending against filling up the disks is made. This would be an important part of a more production-ready solution. The processes always run under the UID of the server. As this is extremely unsafe, the final solution isn't meant to be anything more than a simple proof-of-concept. The UID of the server is assumed to be 0 always due to requirements around the resource constraining - one can't create a new control group as a non-privileged user in Linux. The server does a simple check for the UID to provide a user with friendly error messages. Keeping the list of processes between server restarts is outside of the scope. Scheduled processes are executed immediately, with no ability to specify a point-in-time.

## Technical details

All processes known to the server have a UUID identifying them. This allows the service to distinguish between other processes in the system and the ones spawned by it. It simplifies the logic around assumptions made about the process (like the existence of the stdout and stderr file sinks). The UUID value is initially returned to the client upon the job creation. All other requests require this value as part of the request payload to identify the process in question.

The list of possible request results:

- TLS authentication error
- authorization error
- given process id is not a valid UUID
- (any) task-specific error
- OS unexpected error (when e.g. the OS refuses some operation that the server needs to perform, e.g. writing to log file because the device is full)
- success

The authorization step is very basic and is based on the specific value in the client's certificate subject.

The solution will be coded in Rust, using the latest versions of gRPC and TLS libraries: tonic and rustls. Simple and robust command arguments handling will be provided by the structopt crate. Additional dependencies will be chosen at a later point.

### Task: Start a process

- Arguments:
  - Command name (string)
  - Command arguments (list of strings)
  - Max memory (integer that is greater than zero)
  - Max CPU (integer that is greater than zero)
  - Disk major (integer that is greater than zero)
  - Disk minor (integer that is greater than zero)
  - Max disk IO (integer that is greater than zero)
- Errors:
  - Given command name is empty
  - One of the given command arguments is empty
  - Invalid max memory value
  - Invalid max CPU value
  - Invalid disk minor
  - Invalid disk major
  - Invalid max disk IO value
  - Couldn't start a process
- Returns:
  - A UUID value of the scheduled job

When the process is started, the server adds it (in a thread-safe way) to its internal hash map of processes. The map is keyed with id and valued with the PID. Additionally, two files are created on disk: for storing the stdout and stderr. Each scheduled process has stdout and stderr pointed at these two files. A huge drawback to this solution is that the logs could potentially take up all of the disk space. Handling the corner cases around it is outside the scope of this work.

Upon the process creation, a new control group is created and configured as per the constraint parameters. The new process is added to the group before the server responds with the UUID. 

### Task: Stop a process

- Arguments:
  - A UUID of the process (UUID formatted as a string)
- Errors:
  - Process not found
  - Process already stopped
  - Couldn't stop a process
- Returns:
  - Acknowledgment of the process being stopped

The server tries to kill the process. It sends a SIGTERM signal and waits for 5 seconds. If unsuccessful at this point, it sends a SIGKILL signal. When successful in the end, it returns an acknowledgment - otherwise an error message.

### Task: Query process status

- Arguments:
  - A UUID of the process (UUID formatted as a string)
- Errors:
  - Process not found
- Returns:
  - One of the two values: Running, Stopped

The server reads the process PID from the internal map. It then uses it to query the OS for the process status. When found, it returns the payload that means "process running", and "process stopped" otherwise. A fuller implementation would use a thread-safe on-disk data storage for the hashmap of the processes. This would allow the handling of processes across the server restarts.

### Task: Show the process output

- Arguments:
  - A UUID of the process (UUID formatted as a string)
  - A file descriptor to use: Stdout, Stderr (string)
- Errors:
  - Process not found
- Returns:
  - A stream of the process output at a given file descriptor. The stream is "followed" until Ctrl-C is used in the client.

The server reads from the relevant output file, streaming the contents via gRPC. When no new data is encountered it sleeps for a couple of hundreds of milliseconds and tries to poll for new data. It all happens in a loop and only stops upon the Ctrl-C from the user, which closes a connection. The file-handle is released on the server when the connection is closed. The process output streams a struct that holds the new data and potentially the error message. When an unexpected error happens during the data polling, an error message is streamed back to the client and the connection is closed.
