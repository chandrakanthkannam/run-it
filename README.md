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

## TODO

- Valid unit tests
- Ability to upload a script and run it
