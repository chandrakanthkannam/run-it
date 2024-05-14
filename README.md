# run-it

Run commands on host machine via http request.

## Features

- Core feature is, to run a command on the host and command is submitted via a `http` request
- Application records the commands its running
- Couple of APIs are implemented
  - `/api/submitcmd` -> to submit a command via `POST` method
  - `/api/getcmdstatus/:cmd_id` -> to get status of the command, `cmd_id` would be the response of the `POST` request
- Logging/Tracing is implemented to track the command flow
- Live commands outputs are also captured, a configurable timeout setting for long running commands

## Configurable options

- Below are the environment variables which can be set to overwrite default values
  - `RUST_LOG`: determine log level, Default is INFO.
  - `R_CMD_TIMEOUT`: Time out value in sec, only numerics, example: 100 for 100sec timeout, Default is 50
  - `RUN_IT_PORT`: port number to listen on, Default is 48786

## DEMO

https://github.com/chandrakanthkannam/run-it/assets/49658217/c6e55c75-6eeb-4a94-9786-6dace5cf2404

## How to

### Build locally

- To build locally it is required to have `rust` and `cargo` installed
- Clone the repo and run: `cargo build -r`
- This would leave a executable in `<clone_repo_loc>/target/release/run-it` path
- If above environment variables are set those values are used otherwise default values are used.

## TODO

- Valid unit tests
- Ability to upload a script and run it
