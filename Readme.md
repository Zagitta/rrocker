# Rusty Remote Docker
A rust implementation of [Teleport's system worker coding challenge](https://github.com/gravitational/careers).

Do us both a favour and **never** use this in production 😅

## Overview
This implementation is split into 3 crates:
- rrockerd: The server/daemon that implements the work scheduler and gRPC service.
- rrocker-cli: The CLI client that uses the gRPC API to request work to be scheduled by the server.
- rrocker-lib: The shared API between the client and daemon.



## Approach
The initial document describing the design/approach can be found [here](Approach.md)