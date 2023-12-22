# ansible-sshman

Write a simple yaml file and let ansible set up SSH access to your servers.

## How it works

Reads a yaml config file that lists users and which hosts to give them access to.
Generates a playbook and runs it with `ansible-playbook` or writes it to a file.
The playbook creates accounts for each user on the hosts they have access to and adds their listed public key to their authorised list.
It keeps track of accounts it has created, and disables them if they have been removed from the config.

This tool will never delete users or their data.

### Roles

Users can have one of three possible roles:
+ `user` : Normal user that cannot use sudo.
+ `sudoer` : Normal user that can use sudo.
+ `superuser` : User with UID 0 — equivalent to root.

## Config format

```yaml
- name: Username of user
  pubkeys:
    - Array of
    - public keys
    - the user may use to login.
  access: Ansible group pattern matching hosts this user should have access to.
  role: Controls the privileges a user has on the host. One of the roles listed above.
```

## Usage Help

```
Tool for managing SSH access to machines with Ansible.

Usage: ansible-sshman --config <CONFIG> --inventory <INVENTORY> <COMMAND>

Commands:
  run    Generates and runs the playbook immediately
  write  Writes the playbook to a file
  help   Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>        Path to ssh config file
  -i, --inventory <INVENTORY>  Path to Ansible inventory file
  -h, --help                   Print help
  -V, --version                Print version
```
