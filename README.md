# run-it

Run a command on server via this web interface, just run it

## Requirements

### UI:

- buttons giving ability to select which shell to select
- text block which accepts user commands
- format the input according to selected shell, and save it to the file
  - this file has to be saved into `.cache`
  - filename should be hashed and unique, use time to create the hash
  - file extension should be as per the shell selected
- another button to execute the command
- below the execute button, rest of screen should show history (may be past 10 command) and current running one
  - current running command should show any animation at the end of the row
  - current running commands can be multiple from different sessions
- completed command should have a `uptick` or `red-cross` mark at end of the row
- upon clicking on the command pop-up screen should show the output
