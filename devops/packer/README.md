# General information

_[Return](../README.md)_

[Packer](https://packer.io) can be used to create images with backend and client binaries.
All required extra software is installed and extra AVS repositories are downloded.

# Usage

## Git Hub actions

Packer has been turn off in the GHA, but the client workflow was just commented out, not removed, so can be reused in the future.
Check the `build-image` job in the [Release Client](../github/workflows/release-client.yml) workflow.
The images can be still build manually.
Check the [Packer build notes](#packer-build-notes) section.

## Setting up VM

The VM can be created with a gcloud command.
See example below.
The VM type (n2-standard-2) in the example is cheap, but slow when building rust projects.

* Prepare a few variables. First VM name,
```
VMNAME=ivy-c
```
* next version of cloudstation (I plan to find a way to call 'latest')
```
VERSION=4
```
* and finally the tag (or tags) to set firewall
```
TAG=holesky-eigenda
```
* build machine
```
gcloud compute instances create --image ivynet-cloudstation-$VERSION  --zone "us-central1-a" --boot-disk-size 40GB --machine-type n2-standard-2  $VMNAME
```
* add the tag to open the firewall
```
gcloud compute instances add-tags $VMNAME --tags $TAG
```

### SSH
THE GCP CLI can be also use to login to the VM (because we set OS Login, [see more](https://cloud.google.com/compute/docs/oslogin))
```
gcloud compute ssh --zone "us-central1-a"  --project "ivynet-tests" $VMNAME
```
As well as coping files. E.g.:
```
gcloud compute scp  --zone "us-central1-a"  --project "ivynet-tests" metadata.json  $VMNAME:
```
The VM should have a network tag _holeksy-eigenda_ to enable firewall setup.
I don't know how to do this from commandline, yet.


## On the machine
* The `/opt/ivynet/bin` directory is added to the $PATH variable.
* On the client VM all extra repositories store in the `/opt/ivynet/resources` are owned by the `ivynet` user.
It might be the easiest to use this user for any build process.
```
sudo -i -u ivynet
```
* The same is true for the `ivynet-client` system script.
* The backend services (`ivynet-api`, `ivynet-ingress`, `ivynet-scanner`) are owned and run by `root`.

### MOTD

Extra information are visible after the login.
Check the [motd.txt](../ansible/roles/ivynet-client/templates/motd.txt.j2)

# Packer build notes

* Download plugin for GCP VM
```
packer init cloudstation.pkr.hcl
```
* You need a private key for packer to clone ivynet repo.
* To build pass the version number e.g.
```
packer build -var 'version=2' cloudstation.pkr.hcl
```

## Ansible

Please check the [README](../ansible/README.md) file for more information.

_[Return](../README.md)_
