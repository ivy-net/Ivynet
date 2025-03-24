# DevOps tools

## Pre-Commit
PreCommit is used to ensure code conforming to basic rules.
For example ensure lack of trailing white spaced, but also passing the cargo clippy checks.
Further information about pre-commit usage in the ivynet repo can be found in the [PreCommit](./PreCommit.md) document.

## Ansible

Ansible is the tools used to configure backend and client.
It is a core of many github actions [workflows](github/workflows).
Information about Ansible roles, playbooks and script helping to run them are in [Ansible](ansible/README.md) folder.


## Terraform/OpenTofu

[OpenTofu](https://opentofu.org/) (an open source fork of [Terraform](https://www.terraform.io/)) is used to set the cloud infrastructure.
It is not part of this repo, but rather there are 2 repos for the IaC (Infrastructure as Code).
* The [Open Tofu Modules](https://github.com/ivy-net/otofu-modules) is a collection of modules used to setup backend/api deployments.
* The [Infrastructure](https://github.com/ivy-net/infra) keeps the current state of the cloud infrastructure.
It uses the modules from the former repo.

Having 2 repos allows to version modules without dedicated modules repository.

## Packer

[Packer](https://www.packer.io/) uses Ansible roles to prepare VM images for backend and client (cloudstation).
It was turn off from the GHA to limit workflows costs.
In the future it can reestablish and/or used to create docker images.
More info in the [Packer README](packer/README.md) file.

## Scripts

Most of the scripts were moved to Ansible role.
The only one left is an one-liner to move a client binaries from internal to public GCP bucket.

* [client_copy.sh](scripts/client_copy.sh)
