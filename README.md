# ansible-sshman

Write a simple yaml file and let Ansible set up SSH access to your servers.

## Requirements

+ Ansible
+ The `ansible.posix` collection (`ansible-galaxy collection install ansible.posix`)

## How it works

Reads a yaml config file that lists users and which hosts to give them access to.
Generates a playbook and runs it with `ansible-playbook` or writes it to a file.
The playbook creates accounts for each user on the hosts they have access to and adds their listed public key to their authorised list.
This tool will never delete users or their data. Accounts will be created for users that aren't `blocked`.

### Roles

Users can have one of four possible roles:
+ `blocked` : Cannot login using [publickey authentication](https://www.ssh.com/academy/ssh/public-key-authentication).
+ `sudoer` : Normal user that can use sudo.
+ `superuser` : User with UID 0 — equivalent to root.

### Details

The first play of the playbook contains tasks for creating the groups this tool relies on.
+ The `sshman-sudoer` group will be created alongside a file of the same name under `/etc/suoders.d/`. This group will have sudo permissions `ALL=ALL`.

Next in the playbook will be a play for each user, creating their account on hosts they have access to (unless they are `blocked` - these users will not have accounts created for them).

Finally, a play for each user authorizing their respective keys on hosts they have access to — or removing all keys, for `blocked` users.

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
