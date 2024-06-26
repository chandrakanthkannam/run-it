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

### Run it

- To run this on a linux environment as systemd service, please follow the steps [here](./docs/linux-setup.md)

### Submit commands

- Service has an api exposed, `/api/submitcmd`, send a `POST` request with data in json format, hence need the header `'Content-Type: application/json'`

  - data has can have these keys

  ```
  {
    "cmd": string<required>,
    "args": string<optional>,
    "is_shell": bool<optional>
  }
  ```

  - cmd - is the command need to run, string type
  - args - if the command accepts any args, string type
  - is_shell - if the `cmd` passed is a shell like `bash` or `sh` and args have the actual task, then this need to be set `true`, yes its bool type

  - examples:
    - `curl http://<IP_ADDRESS>:48786/api/submitcmd -X POST -H 'Content-Type: application/json' -d '{"cmd": "whoami"}'`
      - this is a basic example runs cmd `whoami` on the host and request responds with a unique id which can be used to lookup the results
    - `curl http://<IP_ADDRESS>:48786/api/submitcmd -X POST -H 'Content-Type: application/json' -d '{"cmd": "python3", "args": "--version"}'`
      - this is running a pyton command and with an arg, if host doesn't have python installed command would fail and its corresponding output is available on the `/api/getcmdstatus` api
    - `curl http://<IP_ADDRESS>:48786/api/submitcmd -X POST -H 'Content-Type: application/json' -d '{"cmd": "bash", "args": "c=0; while true; do let c=c+1; sleep 10; echo $c; done", "is_shell": true}'`
      - this is a final variant, cmd is shell, and args have actual single line script, in this scenario set the `is_shell` to true. and if you notice the script is set to run forever, in these long running session cmd will timeout after the configured setting.

### Get command status

- Service also provides an API to get the command status, to get the status request a `GET` call on `/api/getcmdstatus/<CMD_ID>`, yes path should have the command ID that `POST` call responded with.
  - example:
    - `curl http://<IP_ADDRESS>:48786/api/getcmdstatus/13995226725389956087 -X GET | jq` as the response is json it can be piped to `jq`
    ```
    {
      "state": "",
      "output": ""
    }
    ```
    If the CMD_ID is not found an empty output is returned otherwise it will have the state and output at that point of time, i mean if the cmd is still active it would return that point in time output and state would be `in-progress`

## TODO

- Valid unit tests
- Ability to upload a script and run it
- external datastore
- server/clinet arch
  - server will have an UI
  - server can not accept commands via api
  - client may or may not an ui module
