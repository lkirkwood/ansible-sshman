# ansible-sshman

This tool aims to make managing SSH access easier for those using ansible.

## How it works

Reads a yaml config file that defines users and their access to the hosts in the ansible inventory.
Generates a playbook and runs it with the installed ansible instance, creating accounts for each user on the hosts they have access to.
Also keeps track of accounts it has created, and disables them if they have been removed from the config.

## Config format

```
- name: Username of user
  pubkeys:
    - Array of
    - public keys
    - the user may use to login.
  access: Ansible group pattern matching hosts this user should have access to.
  sudoer: Whether this user should be able to use sudo on the hosts.
```
