# General information

[Ansible](https://ansible.readthedocs.io/) is used to automate configuration of backend and client Ivynet VMs.
In this directory there are roles to install each ivynet componsent, but also related playbooks and scripts to simplify usage.

Additioanlly to content of this folder, Ivynet provides a public role to [install client](https://github.com/ivy-net/ivynet-client-ansible).

# Roles

* [ivynet-api](roles/ivynet-api) -- api program + third party tools (memcached, postgresql) and systemd config
* [ivynet-client](roles/ivynet-client) -- base for cloudstation - clients binaries, but also rust and systemd config
* [ivynet-ingress](roles/ivynet-ingress) - ingress + SSL setup (cert), other tools and configs
* [ivynet-scanner](roles/ivynet-scanner) - scanner + third party and systemd configs

Every role has two paths to install binaries.
The first one is for a released software (client takes the latest release).
The second one grabs software from the master.

# Inventory

The [gcp.yml](gcp.yml) file is the [dynamic inventory](https://docs.ansible.com/ansible/latest/inventory_guide/intro_dynamic_inventory.html) for GCP.


# Playbooks

## Backend

Each service has a dedicated playbook (`api.yml`, `ingress.yml`, `scanner.yml`).
Additionally, there are two script to use the roles to configure an APIx backend server.

* The `api-server.yml` playbook is used by GHA to configure a server with a release software (e.g. API2 updated by [release-api](../github/workflows/release-api.yml), [release-ingress](../github/workflows/release-ingress.yml), [release-scanner](../github/workflows/release-scanner.yml)).
* The `backend-master.yml` one is a part of a [PR merge into master workflow](../github/workflows/master-pr.yml) (updating API3).

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
In case of api, postgres cannot be properly started (systemd).

### Client test
```
ANSIBLE_VAULT_PASSWORD_FILE=$HOME/.vault.txt molecule converge -- --skip-tags gcp
```

### API test

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
* Prepare proper tests for molecule
* Start to export binaries outside of GitHub (it's going to make roles much easier)
