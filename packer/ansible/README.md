# Roles

* ivynet-client -- base for cloudstation - clients binaries, but also rust
* ivynet-backend -- backend program + third party tools (memcached, postgresql)

## Vault
For now I put the github tokens into defaults for both roles.
That makes easier to run tests and packer.

## Testing

Default molecule scenario cannot test everything.
For client, it cannot download point files.
In case of backend, postgres cannot be properly started (systemd).

### Client test
```
 ANSIBLE_VAULT_PASSWORD_FILE=$HOME/.vault.txt molecule converge --skip-tags gcp
```

### Backend test
```
 ANSIBLE_VAULT_PASSWORD_FILE=$HOME/.vault.txt molecule converge --skip-tags db-config
```

## TODO

* Check why sometime molecule does not work with GCP
* Prepare proper tests for molecule
* Start to export binaries outside of GitHub (it's going to make roles much easier)
