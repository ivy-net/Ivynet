# Roles

* ivynet-client -- base for cloudstation - clients binaries, but also rust
* ivynet-backend -- backend program + third party tools (memcached, postgresql)

## Config
The ansible config
```
[defaults]
stdout_callback = yaml
pipelining = True
```
Pipelining helps to deal with become postgres user.

## Vault
For now I put the github tokens into defaults for both roles.
That makes easier to run tests and packer.

## Testing

Default molecule scenario cannot test everything.
For client, it cannot download point files.
In case of backend, postgres cannot be properly started (systemd).

### Client test
```
ANSIBLE_VAULT_PASSWORD_FILE=$HOME/.vault.txt molecule converge -- --skip-tags gcp
```

### Backend test

* simple test with docker
```
ANSIBLE_VAULT_PASSWORD_FILE=$HOME/.vault.txt molecule converge -- --skip-tags db-config,systemd
```
* full test with GCE instance
```
ANSIBLE_PIPELINING=true ANSIBLE_VAULT_PASSWORD_FILE=$HOME/.vault.txt molecule test -s gce
```
## TODO

* ensure that client is idempotent
* Check why sometime molecule does not work with GCP
* Prepare proper tests for molecule
* Start to export binaries outside of GitHub (it's going to make roles much easier)
