# Scope

As per the challenge requirements I'll here briefly describe high level design choices, scope, tradeoffs and edgecases.

I'll be aiming to implement the 5th level of the challenge.

From a high level perspective I'll be implementing the following components:

- A service/daemon that implements an async gRPC server with authentication and authorization. From here on referred to as `rrockerd`
- A worker library that can start/stop/query/stream fully isolated tasks with resource control. This will be a module of `rrockerd` as it's specialized enough that it's unlikely to be useful as a standalone library.
- A simple CLI with commands to start/stop/query/stream tasks using `rrockerd`. From here on reffered to as `rrocker-cli`
- A shared gRPC api that will be a standalone library that's easy to consume from both `rrockerd` and `rrocker-cli`. From here on referred to as `rrocker-lib`

Here's a breakdown of the engineering tradeoffs for each component:

## rrockerd:
I'll be using the rust crate [nix](https://github.com/nix-rust/nix) extensively for the low level systemcalls. Since nix is merely a safe wrapper around libc I believe this is in the spirit of the challenge.

The daemon will be run entirely in memory and will not persist any state in order to survive reboots/crashes.

The gRPC API is assumed to be internal only, as such there won't be any rate limiting, user action logging or abuse mitigations.

Dealing with tty, ptty, terminals and what not is entirely out of scope and as such any program output will be streamed as the raw bytes written to the stdout and stderr pipes.
Furthermore the output will be chunked on newlines and any lines longer than max gRPC packet size will be truncated.

Resource limits will be implemented using separate cgroups for each

Isolation will be hardcoded to fully isolate each scheduled task from other tasks and the host system. That means a separate PID, Mount, User and Network namespace for each task with no way to connect them.
There will be no network devices attached to any tasks.

Mounting is also considered out of scope and as such each task will be chrooted to a RW root that's a copy of a base image that's removed upon exit of the task.

I'll make no attempt at Linux backwards compatibility and the minimum kernel version will be 5.0+.

Tasks will inherit the UID/GID of the daemon and internally be mapped as root.

For the gRPC implementation I'll be using the rust crate [tonic](https://github.com/hyperium/tonic) to avoid reinventing the wheel.

Mutable interaction with tasks will be mutually exclusive meaning that the stop action will take a write-only lock on the task such that any concurrent stop task will wait for the first to complete and the subsequently fail as the task has already stopped.
Read-only interactions such as query and stream can happen concurrently without issue.

## rrocker-cli:
The CLI will need to be run once per command, so scheduling multiple tasks requires multiple invocations.

There'll be zero command line switches and as such it'll be hardcoded to connect to `rrockerd` running on localhost.

## rrocker-lib:
No attention will be paid to backwards compatibility of the API, meaning no versioning or abstractions.

Human friendliness of the API is a distant afterthought, as such full UUIDs will be required to interact with tasks.

Resource limits will be exposed 1-to-1 with the underlying cgroups API leaving it up to the `rrocker-cli` to expose it in a human friendly way.
# Security
Security is naturally an important aspect of the implementation however as my experience with crypto is limited I'll attempt to follow best practices and use the default configuration of OpenSSL and rustTLS wherever applicable.
In an actual production system you'd have a security expert decide the configuration based on a threat model.

## Authentication
Authentication will be done with mTLS according to the challenge rules.

The following certificates will be created:
- Root Certificate Authority (CA). This is our root of trust and should be stored on an air-gapped system that's only used to sign/revoke the server/client CAs.
- Separate server and client CAs. These are used to sign/revoke server and client certs. Furthermore they're separated such that an infrastructure team is able to own their CA and deploy new servers independently. These CAs also introduce an indirection such that servers and clients don't need to know about all clients and servers but can simply verify an identity has been signed by the server/client CA.
- Server_1 cert used by the server to auth with the clients
- Client_1 and Client_2 certs used to demo user's only being able to see their own tasks
- Admin_1 cert used to demo admin's being able to see all tasks
- An untrusted CA + client cert to demo unauthorized client's can't connect

Private keys will be generated with OpenSSL using the prime256v1 ECDH curve as recommended by mozilla: https://wiki.mozilla.org/Security/Server_Side_TLS#Modern_compatibility.

## Authorization
The authorization scheme will be super simple solution where an authenticated user either is an admin or regular user.
A regular user will only be able to perform actions on it's own tasks while an admin can interact with **all** tasks.

This distinction will be done by looking at the Common Name of the client certificate, `CN=client` for clients and `CN=admin` for admins.
The public key of a cert will be used as the client id despite the drawback that a single client would have to share the same cert across multiple servers against best practices. 
In a production environment you'd introduce another indirection such a user could have multiple certs all manage the same tasks and possibly some way to further segment these tasks into groups.

# Testing
I'll primarily be writing unit and integrations tests with a focus on testing the security.

As such authentication and authorization will be tested to ensure client's can't see each others tasks and that invalid certificates can't authenticate.

I'll also write unit tests for the work scheduler to ensure tasks are properly isolated with their own PID, user, mount and network namespaces.

# Timeline

- PR #0 (~2½ hours):
    1. This design document
- PR #1 (~6 hours):
    1. Setup project structure, build system and expected dependencies
    1. Create the initial .proto API 
    1. Write script to generate dev certificates
    1. Create gRPC scaffolding + authentication for the client library and server
    1. Implement authentication tests for gRPC server
- PR #2 (~10 hours):
    1. Implement/fix feedback given on PR #1
    1. Implement work scheduler and tests
    1. Finish gRPC server implementation using the work scheduler
- PR #3 (~6 hours)
    1. Implement/fix feedback given on PR #2
    1. Fully implement CLI parser
    1. Fix bugs