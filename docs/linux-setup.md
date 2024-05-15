## Running the service on Linux server

This document will let you configure `run-it` as a systemd service

Steps:

- Either build [locally](../README.md#build-locally) or download the artifact from release version if it matches the host machine arch.
- Once executable is downloaded on to host, copy the path to executable.
- Place this below unit file in systemd location, `/usr/lib/systemd/system/`, file name `run-it.service`

  - file content:

  ```
  [Unit]
  Description=Run-it
  After=network.target network-online.target
  Requires=network-online.target

  [Service]
  Type=exec
  ExecStart=<PATH_TO_EXEC>
  PrivateTmp=true
  ProtectSystem=true
  Restart=always
  RestartSec=15s

  [Install]
  WantedBy=multi-user.target

  ```

  To set any [configurable environment variables](../README.md#configurable-options), they can be mentioned in the above file under `Service` section, `Environment="<set_env>"`

- Now the run-it can be managed via systemd.
- To tail/watch the logs they are available via `journalctl` as application write logs to `stdout`.
