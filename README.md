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

Users can have one of four possible roles in each access group:
+ `blocked` : Cannot login using [publickey authentication](https://www.ssh.com/academy/ssh/public-key-authentication).
+ `sudoer` : Normal user that can use sudo by entering the password for root. These users have a locked/disabled password.
+ `nopass` : Normal user that can use sudo without entering a password. These users have a locked/disabled password.
+ `superuser` : User with UID 0 — equivalent to root.

### Details

The first play of the playbook contains tasks for creating the `sshman-sudoer` group and authorising members of this group to use sudo with the root password.

After that there is a similar play for creating the `sshman-nopass` group and authorising its members for passwordless sudo.

Next in the playbook will be a play for each user access group, creating their account on hosts in the group with the specified role (unless that role is `blocked` - these users will not have accounts created for them).

Finally, a play for each user access group, authorising their respective keys on hosts in that group — or removing all keys, for `blocked` users.

## Config format

```yaml
- name: Username of user
  pubkeys:
    - Array of
    - public keys
    - the user may use to login.
  access: 
    - hosts: Ansible host pattern e.g. "production:&sydney"
      role: Some role as defined above e.g. "sudoer"
      groups:
        - An optional list of the groups a user should be in.
        - The user won't be removed from their groups if this key is not specified
        - or if the list is empty.
      seuser: An optional SELinux user to assign to the account e.g. "sysadm_u". Wont be changed if the user already exists.
      
```

## Usage Help

```
Tool for managing SSH access to machines with Ansible.

Usage: ansible-sshman --config <CONFIG> <COMMAND>

Commands:
  run    Generates and runs the playbook immediately
  write  Writes the playbook to a file
  help   Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>        Path to ssh config file
  -h, --help                   Print help
  -V, --version                Print version
```
