Headless server setup
=====================

There are several moving parts that cooperate to facilitate running this machine as a headless server.

1. On startup, we either connect to a preconfigured wifi network or run our own hotspot. To change this setting and reboot immediately, run `net.sh wifi` or `net.sh hotspot` respectively (sudoers is setup to allow the specific commands it runs with no password). To change the wifi network configuration, use Network Manager (make sure one and only one network is set to connect automatically).
    In the case of a hotspot, the IP address is always 10.0.0.1. In the case of wifi, we ping a preconfigured URL that saves the remote IP, which we can then go look up from another computer. Systemd does not have a reliable way to wait for Network Manager to bring up the network, so the script loops until it gets through.
2. The supervisor script runs the main software in a dedicated `screen`. If it crashes or quits, it is restarted. This is controlled through a special "keepalive" file -- if this file is deleted, the supervisor exits instead of restarting the software.
    Also, the supervisor never starts the main software in the first place if the keepalive file exists. This allows the user to control things manually instead. Therefore, to restart the machine and have the software managed by the supervisor, be sure to delete the keepalive file.
3. The main software immediately starts a web server on port 3000. From there the machine may be powered off or restarted (when doing this, it deletes the keepalive file, ensuring that it will be started again when the machine turns on).


This directory contains some symlinks and copies of files that are part of the process:

- `supervisor.sh` [symlink from parent dir]: script run automatically by systemd
- `nri.service` [symlink from `/etc/systemd/system`]: unit file for running `supervisor.sh`
- `ping.php`: goes on the external server, should be accessible at the URL specified by `$SERVER` in `supervisor.sh`
- `ping.sh` [symlink from `/etc/NetworkManager/dispatchers.d`]: updates ping.php whenever the IP changes (must be owned by root and unwritable by anyone else)
- `net.sh` [symlink from parent dir]: script for switching networks
- `nopasswd_net_reboot` [symlink from `/etc/sudoers.d`]: allows commands in `net.sh` (and the shutdown/reboot buttons in the web interface) to be run with no password

