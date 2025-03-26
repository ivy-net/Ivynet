# General information

[Ansible](https://ansible.readthedocs.io/) is used to automate configuration of backend and client Ivynet VMs.
In this directory there are roles to install each ivynet components, but also related playbooks and scripts to simplify Ansible usage.

Additionally to content of this folder, Ivynet provides a public role to [install client](https://github.com/ivy-net/ivynet-client-ansible) from the public bucket.

Finally, Ansible scripts are also present in the [infra](https://github.com/ivy-net/infra) repository.
Check the [README.md](https://github.com/ivy-net/infra/blob/master/Ansible/README.md) file there.

# Support scripts

There is a few shell script helping to run Ivynet Ansible.


All of them check if Ansible is activated first.
If not, script will try to load the Python virtual environment located in the  `~/bin/ansible` directory.
The scripts also load the vault password from the `~/.vault.txt` file.

## api1.sh

The script updates backend services in the production (api1).
It requires a version for each of 3 services:
```
./api1.sh -b 0.5.5 -i 0.5.5 -s 0.5.0
```
The script applies the `api1.yml` playbook at the `api1` host.
(It manipulates the `hosts` line in the playbook to ensure that it configure the right machine.)

## api_update.sh

This script is used by the GH actions ([release-api](../github/workflows/release-api.yml), [release-ingress](../github/workflows/release-ingress.yml), [release-scanner](../github/workflows/release-scanner.yml)) to update a release software in staging environment (API2).
It requires the name of the services as the only parameter and finds the version in the appropriate `Cargo.toml` file.
```
./api_update.sh scanner
```
The script applies the playbook on VMs with the labels `area:backend` and `env:gha`.

## client_update.sh

The scripts install (updates) client software.
If the option `-m` is passed it uploads the latest binaries from the master branch to the VMs with `env:dev` label.
Otherwise it takes the latest release.
```
./client_update.sh -m
```
It works on VMs with the `area:client` label.

# Roles

* [ivynet-api](roles/ivynet-api) -- api program + third party tools (memcached, postgresql) and systemd
* [ivynet-client](roles/ivynet-client) -- base for cloudstation - clients binaries, but also rust and systemd
* [ivynet-ingress](roles/ivynet-ingress) - ingress + SSL setup (cert), other tools and configurations
* [ivynet-scanner](roles/ivynet-scanner) - scanner + third party and systemd

Every role has two ways to install binaries.
The first one is for a released software (client takes the latest release).
The second one grabs software from the master.

# Inventory

The [gcp.yml](gcp.yml) file is the [dynamic inventory](https://docs.ansible.com/ansible/latest/inventory_guide/intro_dynamic_inventory.html) for GCP.

The inventory creates host groups based on GCP labels.
Following one are important:

- `area` (`client`, `backend`)
- `env` (`gha`, `dev`)

The few host specific variables are defined in the [host_vars](host_vars/) folder.

# Playbooks

## Backend

Each service has a dedicated playbook (`api.yml`, `ingress.yml`, `scanner.yml`).
They are used by GHA as mentioned in the [scripts](#client_update.sh) section.

Additionally, there are two playbooks to use the roles to configure an APIx backend server.

* The `api-server.yml` playbook can be used to configure a server with all backend components (using release version) in a staging environment (API2).
* The `backend-master.yml` one is a part of a [PR merge into master workflow](../github/workflows/master-pr.yml) updating a dev environment API3).

# Other information

## Configuration

The Ansible configuration (`~/.ansible.cfg`) file used by me:

```
[defaults]
stdout_callback = yaml
pipelining = True
```

Pipelining helps to deal with tasks requiring to 'become' postgres user.

## Vault

The roles requires the vault password to decrypt some of the variables.
To make it easier for manual, but also GHA automated runs, scripts assumes that the password is stored in the `~/.vault.txt` file.

_Alternative, to using vault is to provide new values from outside of code repo._


## Testing

There are molecule scenario for the `ivynet-client` and the `ivynet-api` roles.

Default molecule scenario cannot test everything.
For client, it cannot download point files from GCP (unless the right credentaials would be passed) and systemd cannot be setup.
In case of api, postgres cannot be properly started (systemd) and systemd service cannot be setup.

To run at least some test tasks with some tags can be skipped.

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

# TODO

* molecule in every role
* molecule scenario to test systemd locally (e.g. Vagrant, or dedicated docker image)
* use GHA to test roles
* test GHA access with appropriate crendetials
