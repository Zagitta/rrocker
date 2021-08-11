# Rusty Remote Docker
A rust implementation of a minimal docker with a gRPC interface. 
Built as part of a job interview/challenge that didn't succeed so your milage may vary 😉

## Overview
This implementation is split into 3 crates:
- rrockerd: The server/daemon that implements the work scheduler and gRPC service.
- rrocker-cli: The CLI client that uses the gRPC API to request work to be scheduled by the server.
- rrocker-lib: The shared API between the client and daemon.



## Approach
The initial document describing the design/approach can be found [here](Approach.md)